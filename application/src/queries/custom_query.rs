use async_trait::async_trait;
use cqrs_es::persist::{PersistenceError, ViewContext, ViewRepository};
use cqrs_es::{Aggregate, EventEnvelope, Query, View};

/// A custom query trait. This trait is used to define custom queries for the Aggregates that do not make use of
/// `GenericQuery`.
#[async_trait]
pub trait CustomQuery<R, V, A>: Query<A>
where
    R: ViewRepository<V, A>,
    V: View<A>,
    A: Aggregate,
{
    async fn load_mut(&self, view_id: String) -> Result<(V, ViewContext), PersistenceError>;

    async fn apply_events(
        &self,
        view_id: &str,
        events: &[EventEnvelope<A>],
    ) -> Result<(), PersistenceError>;
}
