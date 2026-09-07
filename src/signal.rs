//! Signal primitives and effect combinators for The Elm Architecture (TEA).
//!
//! A [`Signal`] represents an outcome produced by a component or application update step.
//! It unifies internal message loops, parent communication, asynchronous runtime tasks,
//! concurrent batches, sequential task chains, and error handling into a single composable type.

use core::future::Future;
use iced::Task;
use iced_futures::MaybeSend;

/// An outcome or effect produced by an update transition.
///
/// Signals allow components to declare both synchronous and asynchronous side-effects,
/// communicate with parent components via outbound messages, and manage failure recovery.
#[derive(Debug, Default)]
pub enum Signal<M, O> {
    /// An internal message to be re-dispatched into the component or app update cycle.
    Message(M),
    /// An outbound notification or event intended for a parent component to handle.
    Out(O),
    /// An asynchronous runtime [`Task`] yielding an internal message upon completion.
    Task(Task<M>),
    /// A collection of signals to be executed concurrently or without strict ordering.
    Batch(Vec<Self>),
    /// A collection of signals to be executed sequentially in strict sequence order.
    Sequence(Vec<Self>),
    /// An error boundary wrapping an inner signal, providing an optional fallback message on failure.
    OnError(Box<Self>, Option<M>),
    /// No-op signal indicating completion with no further actions.
    #[default]
    Done,
}

impl<M, O> Signal<M, O>
where
    M: 'static + MaybeSend,
{
    /// Wraps `self` in a successful [`anyhow::Result`].
    ///
    /// # Errors
    ///
    /// This function is infallible and always returns `Ok(self)`.
    pub const fn ok(self) -> anyhow::Result<Self> {
        Ok(self)
    }

    /// Returns a successful [`Signal::Done`] wrapped in [`anyhow::Result`].
    ///
    /// # Errors
    ///
    /// This function is infallible and always returns `Ok(Signal::Done)`.
    pub const fn done() -> anyhow::Result<Self> {
        Ok(Self::Done)
    }

    /// Returns `true` if this signal is [`Signal::Done`].
    #[must_use]
    pub const fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// Constructs a [`Signal::Message`] from any value convertible into `M`.
    #[must_use]
    pub fn msg<Msg>(message: Msg) -> Self
    where
        Msg: Into<M>,
    {
        Self::Message(message.into())
    }

    /// Constructs a [`Signal::Out`] from any value convertible into `O`.
    #[must_use]
    pub fn out<Out>(message: Out) -> Self
    where
        Out: Into<O>,
    {
        Self::Out(message.into())
    }

    /// Constructs a [`Signal::Task`] wrapping an [`iced::Task`].
    #[must_use]
    pub const fn task(task: Task<M>) -> Self {
        Self::Task(task)
    }

    /// Constructs a delayed [`Signal::Task`] that yields the given message on the runtime task queue.
    #[must_use]
    pub fn delayed<Msg>(message: Msg) -> Self
    where
        Msg: Into<M>,
    {
        Self::Task(Task::done(message.into()))
    }

    /// Performs an asynchronous [`Future`] and maps its completion output into an internal message.
    #[must_use]
    pub fn perform<Fun, Out, H>(future: Fun, handler: H) -> Self
    where
        Fun: Future<Output = Out> + Send + 'static,
        H: Fn(Out) -> M + MaybeSend + 'static,
        Out: MaybeSend + 'static,
    {
        Self::Task(Task::perform(future, handler))
    }

    /// Dispatches an asynchronous [`Future`] that directly produces an internal message.
    #[must_use]
    pub fn future<F>(future: F) -> Self
    where
        F: Future<Output = M> + Send + 'static,
    {
        Self::Task(Task::future(future))
    }

    /// Wraps `self` in an error recovery handler that emits `signal` if processing fails.
    #[must_use]
    pub fn on_error<Msg>(self, signal: Msg) -> Self
    where
        Msg: Into<M>,
    {
        Self::OnError(Box::new(self), Some(signal.into()))
    }

    /// Wraps `self` in an error recovery handler that silently ignores any execution failure.
    #[must_use]
    pub fn ignore_error(self) -> Self {
        Self::OnError(Box::new(self), None)
    }

    /// Wraps `self` in an error recovery handler with an optional fallback message.
    #[must_use]
    pub fn maybe_on_error<Msg>(self, signal: Option<Msg>) -> Self
    where
        Msg: Into<M>,
    {
        Self::OnError(Box::new(self), signal.map(Into::into))
    }

    /// Combines an iterator of signals into a flat [`Signal::Batch`].
    ///
    /// Any empty or [`Signal::Done`] entries are discarded. If the resulting collection
    /// contains only a single signal, that signal is unwrapped directly.
    #[must_use]
    pub fn batch<I>(signals: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        let iter = signals.into_iter();
        let mut flat = Vec::with_capacity(iter.size_hint().0);
        for signal in iter {
            match signal {
                Self::Done => {}
                Self::Batch(items) => flat.extend(items),
                other => flat.push(other),
            }
        }
        match flat.len() {
            0 => Self::Done,
            1 => flat.pop().expect("flat has exactly one element"),
            _ => Self::Batch(flat),
        }
    }

    /// Combines an iterator of signals into a flat [`Signal::Sequence`].
    ///
    /// Any empty or [`Signal::Done`] entries are discarded. If the resulting collection
    /// contains only a single signal, that signal is unwrapped directly.
    #[must_use]
    pub fn sequence<I>(signals: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        let iter = signals.into_iter();
        let mut flat = Vec::with_capacity(iter.size_hint().0);
        for signal in iter {
            match signal {
                Self::Done => {}
                Self::Sequence(items) => flat.extend(items),
                other => flat.push(other),
            }
        }
        match flat.len() {
            0 => Self::Done,
            1 => flat.pop().expect("flat has exactly one element"),
            _ => Self::Sequence(flat),
        }
    }

    /// Chains two signals into a sequential execution order (`self` followed by `other`).
    #[must_use]
    pub fn chain(self, other: Self) -> Self {
        match (self, other) {
            (Self::Done, eff) | (eff, Self::Done) => eff,
            (Self::Sequence(mut left), Self::Sequence(right)) => {
                left.extend(right);
                Self::Sequence(left)
            }
            (signal, Self::Sequence(mut signals)) => {
                signals.insert(0, signal);
                Self::Sequence(signals)
            }
            (Self::Sequence(mut signals), signal) => {
                signals.push(signal);
                Self::Sequence(signals)
            }
            (left, right) => Self::Sequence(vec![left, right]),
        }
    }

    /// Merges two signals into a concurrent or unordered batch.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Done, eff) | (eff, Self::Done) => eff,
            (Self::Batch(mut left), Self::Batch(right)) => {
                left.extend(right);
                Self::Batch(left)
            }
            (signal, Self::Batch(mut signals)) | (Self::Batch(mut signals), signal) => {
                signals.push(signal);
                Self::Batch(signals)
            }
            (left, right) => Self::Batch(vec![left, right]),
        }
    }

    fn inner_map<MN, ON, F>(self, map_out: &F) -> anyhow::Result<Signal<MN, ON>>
    where
        MN: Send + MaybeSend + 'static,
        M: MaybeSend + 'static + Into<MN>,
        F: Fn(O) -> anyhow::Result<Signal<MN, ON>>,
    {
        match self {
            Self::Done => Ok(Signal::Done),
            Self::Message(message) => Ok(Signal::Message(message.into())),
            Self::Out(message) => map_out(message),
            Self::Task(task) => Ok(Signal::Task(task.map(Into::into))),
            Self::Batch(batch) => Ok(Signal::Batch(
                batch
                    .into_iter()
                    .map(|signal| signal.inner_map(map_out))
                    .collect::<anyhow::Result<Vec<_>>>()?,
            )),
            Self::Sequence(sequence) => Ok(Signal::Sequence(
                sequence
                    .into_iter()
                    .map(|signal| signal.inner_map(map_out))
                    .collect::<anyhow::Result<Vec<_>>>()?,
            )),
            Self::OnError(signal, on_error) => Ok(Signal::OnError(
                Box::new(signal.inner_map(map_out)?),
                on_error.map(Into::into),
            )),
        }
    }

    /// Recursively transforms the outbound message `O` into a parent [`Signal`].
    ///
    /// Internal messages `M` are converted into the parent message type `MN` via [`Into`].
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping closure `map_out` returns an error for any outbound event.
    pub fn map<MN, ON, F>(self, map_out: F) -> anyhow::Result<Signal<MN, ON>>
    where
        MN: Send + MaybeSend + 'static,
        M: MaybeSend + 'static + Into<MN>,
        F: Fn(O) -> anyhow::Result<Signal<MN, ON>>,
    {
        self.inner_map(&map_out)
    }
}

impl<M> Signal<M, ()>
where
    M: MaybeSend + 'static,
{
    /// Maps a signal with empty outbound message `()` into a parent [`Signal`], dropping the unit output.
    ///
    /// # Errors
    ///
    /// This function is infallible and returns `Ok(Signal)`.
    pub fn map_empty<MN, ON>(self) -> anyhow::Result<Signal<MN, ON>>
    where
        MN: Send + MaybeSend + 'static,
        M: Into<MN>,
    {
        self.map(|()| Ok(Signal::Done))
    }
}

impl<M, O> From<Task<M>> for Signal<M, O>
where
    M: 'static + MaybeSend,
{
    fn from(task: Task<M>) -> Self {
        Self::Task(task)
    }
}
