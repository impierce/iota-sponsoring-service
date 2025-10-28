use async_trait::async_trait;
use cqrs_es::persist::{PersistenceError, QueryErrorHandler, ViewContext, ViewRepository};
use cqrs_es::{Aggregate, EventEnvelope, Query, View};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::broadcast;

/// A simple query and view repository. This is used both to act as a `Query` for processing events
/// and to return materialized views. It also broadcasts updated views via a `tokio::broadcast::Sender`.
pub struct GenericQueryWithSender<R, V, A>
where
    R: ViewRepository<V, A>,
    V: View<A> + Clone,
    A: Aggregate,
{
    view_repository: Arc<R>,
    error_handler: Option<Box<QueryErrorHandler>>,
    sender: broadcast::Sender<V>,
    phantom: PhantomData<(V, A)>,
}

impl<R, V, A> GenericQueryWithSender<R, V, A>
where
    R: ViewRepository<V, A>,
    V: View<A> + Clone,
    A: Aggregate,
{
    pub fn new(view_repository: Arc<R>, sender: broadcast::Sender<V>) -> Self {
        Self {
            view_repository,
            error_handler: None,
            sender,
            phantom: PhantomData,
        }
    }

    pub fn use_error_handler(&mut self, error_handler: Box<QueryErrorHandler>) {
        self.error_handler = Some(error_handler);
    }

    pub async fn load(&self, view_id: &str) -> Option<V> {
        match self.view_repository.load_with_context(view_id).await {
            Ok(option) => option.map(|(view, _)| view),
            Err(e) => {
                self.handle_error(e);
                None
            }
        }
    }

    async fn load_mut(&self, view_id: String) -> Result<(V, ViewContext), PersistenceError> {
        Ok(self
            .view_repository
            .load_with_context(&view_id)
            .await?
            .unwrap_or_else(|| (Default::default(), ViewContext::new(view_id, 0))))
    }

    pub(crate) async fn apply_events(
        &self,
        view_id: &str,
        events: &[EventEnvelope<A>],
    ) -> Result<(), PersistenceError> {
        let (mut view, view_context) = self.load_mut(view_id.to_string()).await?;
        for event in events {
            view.update(event);
        }
        self.view_repository
            .update_view(view.clone(), view_context)
            .await?;

        let _ = self.sender.send(view);

        Ok(())
    }

    fn handle_error(&self, error: PersistenceError) {
        if let Some(handler) = &self.error_handler {
            (handler)(error);
        }
    }
}

#[async_trait]
impl<R, V, A> Query<A> for GenericQueryWithSender<R, V, A>
where
    R: ViewRepository<V, A>,
    V: View<A> + Clone,
    A: Aggregate,
{
    async fn dispatch(&self, view_id: &str, events: &[EventEnvelope<A>]) {
        if let Err(err) = self.apply_events(view_id, events).await {
            self.handle_error(err);
        };
    }
}
