use super::ComputeChannel;
use crate::server::{Binding, ComputeServer, Handle};
use crate::storage::ComputeStorage;
use alloc::sync::Arc;
use alloc::vec::Vec;
use cubecl_common::reader::Reader;
use cubecl_common::sync_type::SyncType;

/// A channel using a [ref cell](core::cell::RefCell) to access the server with mutability.
///
/// # Important
///
/// Only use this channel if you don't use any threading in your application, otherwise it will
/// panic or cause undefined behaviors.
///
/// This is mosly useful for `no-std` environments where threads aren't supported, otherwise prefer
/// the [mutex](super::MutexComputeChannel) or the [mpsc](super::MpscComputeChannel) channels.
#[derive(Debug)]
pub struct RefCellComputeChannel<Server> {
    server: Arc<core::cell::RefCell<Server>>,
}

impl<S> Clone for RefCellComputeChannel<S> {
    fn clone(&self) -> Self {
        Self {
            server: self.server.clone(),
        }
    }
}

impl<Server> RefCellComputeChannel<Server>
where
    Server: ComputeServer,
{
    /// Create a new cell compute channel.
    pub fn new(server: Server) -> Self {
        Self {
            server: Arc::new(core::cell::RefCell::new(server)),
        }
    }
}

impl<Server> ComputeChannel<Server> for RefCellComputeChannel<Server>
where
    Server: ComputeServer + Send,
{
    fn read(&self, binding: Binding<Server>) -> Reader {
        self.server.borrow_mut().read(binding)
    }

    fn get_resource(
        &self,
        binding: Binding<Server>,
    ) -> <Server::Storage as ComputeStorage>::Resource {
        self.server.borrow_mut().get_resource(binding)
    }

    fn create(&self, resource: &[u8]) -> Handle<Server> {
        self.server.borrow_mut().create(resource)
    }

    fn empty(&self, size: usize) -> Handle<Server> {
        self.server.borrow_mut().empty(size)
    }

    fn execute(
        &self,
        kernel_description: Server::Kernel,
        count: Server::DispatchOptions,
        bindings: Vec<Binding<Server>>,
    ) {
        self.server
            .borrow_mut()
            .execute(kernel_description, count, bindings)
    }

    fn sync(&self, sync_type: SyncType) {
        self.server.borrow_mut().sync(sync_type)
    }
}

/// This is unsafe, since no concurrency is supported by the `RefCell` channel.
/// However using this channel should only be done in single threaded environments such as `no-std`.
unsafe impl<Server: ComputeServer> Send for RefCellComputeChannel<Server> {}
unsafe impl<Server: ComputeServer> Sync for RefCellComputeChannel<Server> {}
