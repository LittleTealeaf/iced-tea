//! Top-level application runtime bridge and signal execution engine.
//!
//! The [`App`] trait integrates Elm models and components into the native [`iced::application()`]
//! runtime, handling window lifecycle, subscriptions, themes, and recursive signal execution.

use core::fmt::Debug;

use iced::theme::Base;
use iced::{Application, Element, Program, Subscription, Task, Theme, application};

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

    /// Returns the active window title.
    fn title(&self) -> String;

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
        eprintln!("Error: {error:#}");
    }

    /// Declares active event subscriptions (timers, keyboard/mouse listeners, websockets, etc.).
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the active UI theme.
    fn theme(&self) -> Theme {
        <Theme as Base>::default(iced::theme::Mode::None)
    }

    /// Returns the application scale factor (default is `1.0`).
    fn scale_factor(&self) -> f32 {
        1.0
    }

    /// Evaluates and executes a [`Signal`], translating it into an executable runtime [`Task`].
    ///
    /// Recursively unpacks batches, chains sequential tasks, re-routes internal messages,
    /// and invokes fallback error handlers.
    ///
    /// # Errors
    ///
    /// Returns an error if signal evaluation fails.
    fn process_signal(&mut self, signal: Signal<Self::Message, ()>) -> anyhow::Result<Task<Self::Message>> {
        match signal {
            Signal::Out(()) | Signal::Done => Ok(Task::none()),
            Signal::Task(task) => Ok(task),
            Signal::Message(message) => self.process_message(message),
            Signal::Batch(signals) => {
                let mut errors = Vec::new();
                let mut tasks = Vec::new();
                for sig in signals {
                    match self.process_signal(sig) {
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
                    task = task.chain(self.process_signal(sig)?);
                }
                Ok(task)
            }
            Signal::OnError(inner, on_error) => {
                let result = self.process_signal(*inner);
                match result {
                    Ok(task) => Ok(task),
                    Err(error) => {
                        eprintln!("Gracefully caught error: {error:?}");
                        on_error.map_or_else(|| Ok(Task::none()), |message| self.process_message(message))
                    }
                }
            }
        }
    }

    /// Dispatches a message through [`App::update`] and evaluates the resulting [`Signal`].
    ///
    /// In debug builds, prints the received message.
    ///
    /// # Errors
    ///
    /// Returns an error if the update or resulting signal execution fails.
    fn process_message(&mut self, message: Self::Message) -> anyhow::Result<Task<Self::Message>> {
        #[cfg(debug_assertions)]
        {
            println!("Msg: {message:?}");
        }
        let signal = self.update(message)?;
        self.process_signal(signal)
    }

    /// Executes an update step and logs any errors to [`App::on_error`].
    fn handle_update(&mut self, message: Self::Message) -> Task<Self::Message> {
        self.process_message(message).unwrap_or_else(|error| {
            self.on_error(&error);
            Task::none()
        })
    }

    /// Builds and configures an [`iced::Application`] ready to be launched via `.run()`.
    fn application() -> Application<impl Program<Message = Self::Message, Theme = Theme>> {
        application(
            || {
                let (app, maybe_task) = Self::boot();
                (app, maybe_task.unwrap_or_else(Task::none))
            },
            Self::handle_update,
            Self::view,
        )
        .title(Self::title)
        .subscription(Self::subscription)
        .theme(Self::theme)
        .scale_factor(Self::scale_factor)
    }
}
