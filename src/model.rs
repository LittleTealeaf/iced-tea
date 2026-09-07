//! State management and Elm update contracts for hierarchically composed components.
//!
//! The [`Model`] trait formalizes component state transitions in The Elm Architecture (TEA).
//! Each model defines its internal `Message`, an `OutMessage` for signaling parents, and
//! an update method returning a [`Signal`].

use iced_futures::MaybeSend;

use crate::signal::Signal;

/// Core Elm Architecture model representing component state and transitions.
///
/// Models process messages within an optional runtime context and produce [`Signal`] outcomes
/// that combine internal updates, external notifications, and asynchronous effects.
pub trait Model {
    /// Internal messages handled directly by this model.
    type Message;
    /// Outbound events emitted to notify parent components.
    type OutMessage;
    /// Ephemeral context borrowed during state transitions.
    type Context<'a>
    where
        Self: 'a;

    /// Updates the model state in response to an incoming message.
    ///
    /// # Errors
    ///
    /// Returns an error if the update operation fails or encounters an invalid state.
    fn update<'a>(
        &'a mut self,
        message: Self::Message,
        context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>>;

    /// Updates the model and translates child outbound events into parent signals via `map_out`.
    ///
    /// # Errors
    ///
    /// Returns an error if the update or the mapping transformation fails.
    fn map_update<'a, M, O, F>(
        &'a mut self,
        message: Self::Message,
        context: Self::Context<'a>,
        map_out: F,
    ) -> anyhow::Result<Signal<M, O>>
    where
        F: Fn(Self::OutMessage) -> anyhow::Result<Signal<M, O>>,
        Self::Message: Into<M> + 'static + MaybeSend,
        M: Send + MaybeSend + 'static,
    {
        self.update(message, context)?.map(map_out)
    }

    /// Updates a child model that produces no outbound messages (`OutMessage = ()`),
    /// lifting child signals into the parent's message domain.
    ///
    /// # Errors
    ///
    /// Returns an error if the child update operation fails.
    fn empty_update<'a, M, O>(
        &'a mut self,
        message: Self::Message,
        context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<M, O>>
    where
        Self: Model<OutMessage = ()>,
        M: Send + MaybeSend + 'static,
        Self::Message: Into<M> + Send + 'static,
    {
        self.update(message, context)?.map_empty()
    }
}

/// Convenience trait for models capable of handling specific message types `M`.
///
/// Provides a uniform abstraction for dispatching messages, automatically implemented
/// for all `T: Model` where `M = T::Message`.
pub trait HandleMessage<M>: Model {
    /// Handles an incoming message `M` within the given context.
    ///
    /// # Errors
    ///
    /// Returns an error if processing the message fails.
    fn handle_message<'a>(
        &'a mut self,
        message: M,
        context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>>;
}

impl<T: Model> HandleMessage<T::Message> for T {
    fn handle_message<'a>(
        &'a mut self,
        message: T::Message,
        context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>> {
        self.update(message, context)
    }
}
