//! Top-level application runtime bridge and signal execution engine.
//!
//! The [`App`] trait integrates Elm models and components into the native [`iced::application()`]
//! runtime, handling window lifecycle, subscriptions, themes, and recursive signal execution.

use core::fmt::Debug;

use iced::{Application, Element, Program, Task, Theme, application};

use crate::signal::Signal;

/// Top-level application contract connecting an Elm root component to Iced's runtime.
///
/// Manages application initialization ([`App::boot`]), titles, rendering, message processing,
/// error handling, event subscriptions, themes, and window scaling.
pub trait App: Sized + 'static {
    /// The top-level message type handled by this application.
    type Message: Send + 'static + Debug;

    /// Boots the application, returning initial state and an optional startup task.
    fn boot() -> (Self, Option<Task<Self::Message>>);

    /// Configuration method to add additional configruations
    fn config(
        app: Application<impl Program<Message = Self::Message, Theme = Theme>>,
    ) -> Application<impl Program<Message = Self::Message, Theme = Theme>>;

    /// Renders the root application interface into an [`Element`].
    fn view(&self) -> Element<'_, Self::Message>;

    /// Processes an incoming message, mutating application state and returning a [`Signal`].
    ///
    /// # Errors
    ///
    /// Returns an error if the update transition fails.
    fn update(&mut self, message: Self::Message) -> anyhow::Result<Signal<Self::Message, ()>>;

    /// Invoked when an update or signal execution error occurs.
    fn on_error(&mut self, error: &anyhow::Error) {
        let _ = error;
    }

    fn run() -> iced::Result {
        let app = application(
            || {
                let (app, maybe_task) = Self::boot();
                (app, maybe_task.unwrap_or_else(Task::none))
            },
            handle_update,
            Self::view,
        );
        let configured = Self::config(app);
        configured.run()
    }
}

/*
 * Private Functions
 */

fn handle_update<T>(app: &mut T, message: T::Message) -> Task<T::Message>
where
    T: App,
{
    process_message(app, message).unwrap_or_else(|error| {
        #[cfg(feature = "logger")]
        log::error!("Error: {error:#}");
        app.on_error(&error);
        Task::none()
    })
}

fn process_signal<T>(app: &mut T, signal: Signal<T::Message, ()>) -> anyhow::Result<Task<T::Message>>
where
    T: App,
{
    match signal {
        Signal::Out(()) | Signal::Done => Ok(Task::none()),
        Signal::Task(task) => Ok(task),
        Signal::Message(message) => process_message(app, message),
        Signal::Batch(signals) => {
            let mut errors = Vec::new();
            let mut tasks = Vec::new();
            for sig in signals {
                match process_signal(app, sig) {
                    Ok(task) => tasks.push(task),
                    Err(error) => errors.push(error),
                }
            }

            if errors.is_empty() {
                Ok(Task::batch(tasks))
            } else {
                Err(anyhow::anyhow!("Multiple Errors Occurred: {errors:?}"))
            }
        }
        Signal::Sequence(signals) => {
            let mut task = Task::none();
            for sig in signals {
                task = task.chain(process_signal(app, sig)?);
            }
            Ok(task)
        }
        Signal::OnError(inner, on_error) => {
            let result = process_signal(app, *inner);
            match result {
                Ok(task) => Ok(task),
                Err(error) => {
                    #[cfg(feature = "logger")]
                    log::debug!("Gracefully Caught Error: {error:#}");
                    on_error.map_or_else(|| Ok(Task::none()), |message| process_message(app, message))
                }
            }
        }
    }
}

fn process_message<T>(app: &mut T, message: T::Message) -> anyhow::Result<Task<T::Message>>
where
    T: App,
{
    #[cfg(feature = "logger")]
    log::debug!("Update: {message:?}");
    let signal = app.update(message)?;
    process_signal(app, signal)
}
