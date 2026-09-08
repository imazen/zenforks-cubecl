use crate::{
    compiler::mlir_engine::MlirEngine,
    compute::{
        runner::KernelRunner,
        schedule::{BindingsResource, ScheduleTask},
    },
};
use cubecl_common::bytes::Bytes;
use cubecl_core::{
    CubeDim,
    server::{ExecutionMode, IoError, ServerError},
};
use cubecl_runtime::{logging::ServerLogger, storage::BytesResource};
use std::sync::{Arc, OnceLock, mpsc::SyncSender};

static INSTANCE: OnceLock<CpuExecutionQueue> = OnceLock::new();

#[derive(Clone)]
/// There is a single execution queue instance for the whole CPU runtime.
///
/// This type allows users to send tasks to the global execution queue.
pub struct CpuExecutionQueue {
    sender: SyncSender<QueueItem>,
}

enum QueueItem {
    Task(ScheduleTask),
    /// Carries back whatever failed since the last flush.
    ///
    /// The execution queue is a process-wide singleton on its own thread, so a task
    /// that fails there has no stream to report to at the time it fails. Errors are
    /// held on the queue server and handed to whoever flushes next, which is how they
    /// reach a caller at all instead of panicking on a detached thread.
    ///
    /// Consequence of that singleton, worth knowing: the flush that collects an error
    /// is not necessarily from the stream whose task produced it. Reporting it on the
    /// wrong stream is still better than the alternative it replaces -- a panic on the
    /// queue thread, which no stream could observe.
    Flush(std::sync::mpsc::SyncSender<Vec<ServerError>>),
}

impl CpuExecutionQueue {
    /// Adds a new task to the queue.
    pub fn add(&self, task: ScheduleTask) {
        self.sender.send(QueueItem::Task(task)).unwrap();
    }

    /// Flushes the queue, making sure all enqueued tasks before this point are executed.
    /// Flushes the queue, making sure all enqueued tasks before this point are
    /// executed, and returns the errors they produced.
    pub fn flush(&self) -> Vec<ServerError> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.sender.send(QueueItem::Flush(sender)).unwrap();
        receiver.recv().unwrap()
    }

    /// Resolves the global execution queue instance.
    pub fn get(logger: Arc<ServerLogger>) -> Self {
        INSTANCE.get_or_init(|| Self::init(logger)).clone()
    }

    fn init(logger: Arc<ServerLogger>) -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel(32);

        std::thread::spawn(move || {
            let mut server = CpuExecutionQueueServer {
                runner: KernelRunner::new(logger),
                errors: Vec::new(),
            };

            loop {
                match receiver.recv() {
                    Ok(item) => match item {
                        QueueItem::Task(task) => server.execute_task(task),
                        QueueItem::Flush(sender) => {
                            sender.send(core::mem::take(&mut server.errors)).unwrap()
                        }
                    },
                    Err(err) => panic!("{err:?}"),
                }
            }
        });

        Self { sender }
    }
}

struct CpuExecutionQueueServer {
    runner: KernelRunner,
    /// Failures since the last flush; drained by [`QueueItem::Flush`].
    errors: Vec<ServerError>,
}

impl CpuExecutionQueueServer {
    fn execute_task(&mut self, task: ScheduleTask) {
        match task {
            ScheduleTask::Write { data, buffer } => self.write(data, buffer),
            ScheduleTask::Execute {
                mlir_engine,
                bindings,
                kind,
                cube_dim,
                cube_count,
            } => {
                if let Err(err) = self.kernel(mlir_engine, bindings, kind, cube_dim, cube_count) {
                    self.errors.push(ServerError::Io(err));
                }
            }
        }
    }

    fn write(&mut self, data: Bytes, mut buffer: BytesResource) {
        buffer.write().copy_from_slice(&data);
    }

    fn kernel(
        &mut self,
        mlir_engine: MlirEngine,
        bindings: BindingsResource,
        kind: ExecutionMode,
        cube_dim: CubeDim,
        cube_count: [u32; 3],
    ) -> Result<(), IoError> {
        self.runner
            .execute_data(mlir_engine, bindings, kind, cube_dim, cube_count)
    }
}
