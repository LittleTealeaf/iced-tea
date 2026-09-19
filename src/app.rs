//! Top-level application runtime bridge and signal execution engine.
//!
//! The [`App`] trait integrates Elm models and components into the native [`iced::application()`]
//! runtime, handling window lifecycle, configuration, themes, and recursive signal execution.

use core::fmt::Debug;

use iced::{Application, Element, Program, Task, Theme, application};

use crate::signal::Signal;

/// Top-level application contract connecting an Elm root component to Iced's runtime.
///
/// Manages application initialization ([`App::boot`]), configuration ([`App::config`]), rendering,
/// message processing, error handling, and runtime launch ([`App::run`]).
pub trait App: Sized + 'static {
    /// The top-level message type handled by this application.
    type Message: Send + 'static + Debug;

    /// Boots the application, returning initial state and an optional startup task.
    fn boot() -> (Self, Option<Task<Self::Message>>);

    /// Configuration method to customize the [`Application`] builder (e.g. title, theme, subscriptions).
    fn config<P>(
        app: Application<P>,
    ) -> Application<impl Program<State = Self, Message = Self::Message, Theme = Theme>>
    where
        P: Program<State = Self, Message = Self::Message, Theme = Theme>,
    {
        app
    }

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
                    #[cfg(not(feature = "logger"))]
                    let _ = &error;
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;

    #[derive(Debug, Default)]
    struct MockApp {
        counter: i32,
        errors: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MockMsg {
        Increment,
        Fail(String),
        BatchTrigger,
        SequenceTrigger,
        FallbackHandled,
    }

    impl App for MockApp {
        type Message = MockMsg;

        fn boot() -> (Self, Option<Task<Self::Message>>) {
            (Self::default(), None)
        }

        fn config<P>(
            app: Application<P>,
        ) -> Application<impl Program<State = Self, Message = Self::Message, Theme = Theme>>
        where
            P: Program<State = Self, Message = Self::Message, Theme = Theme>,
        {
            app.title("Mock App")
        }

        fn view(&self) -> Element<'_, Self::Message> {
            text("mock").into()
        }

        fn update(&mut self, message: Self::Message) -> anyhow::Result<Signal<Self::Message, ()>> {
            match message {
                MockMsg::Increment => {
                    self.counter += 1;
                    Signal::done()
                }
                MockMsg::Fail(err) => Err(anyhow::anyhow!(err)),
                MockMsg::BatchTrigger => Ok(Signal::Batch(vec![
                    Signal::msg(MockMsg::Increment),
                    Signal::msg(MockMsg::Increment),
                ])),
                MockMsg::SequenceTrigger => Ok(Signal::Sequence(vec![
                    Signal::msg(MockMsg::Increment),
                    Signal::msg(MockMsg::Increment),
                ])),
                MockMsg::FallbackHandled => {
                    self.counter += 100;
                    Signal::done()
                }
            }
        }

        fn on_error(&mut self, error: &anyhow::Error) {
            self.errors.push(format!("{error:#}"));
        }
    }

    #[test]
    fn test_process_message_success() {
        let mut app = MockApp::default();
        let _ = process_message(&mut app, MockMsg::Increment).unwrap();
        assert_eq!(app.counter, 1);
    }

    #[test]
    fn test_process_message_failure() {
        let mut app = MockApp::default();
        let res = process_message(&mut app, MockMsg::Fail("failed".to_owned()));
        assert_eq!(res.unwrap_err().to_string(), "failed");
        assert_eq!(app.counter, 0);
    }

    #[test]
    fn test_process_signal_simple_variants() {
        let mut app = MockApp::default();
        let _ = process_signal(&mut app, Signal::Done).unwrap();
        let _ = process_signal(&mut app, Signal::Out(())).unwrap();
        let _ = process_signal(&mut app, Signal::Task(Task::none())).unwrap();
        let _ = process_signal(&mut app, Signal::msg(MockMsg::Increment)).unwrap();
        assert_eq!(app.counter, 1);
    }

    #[test]
    fn test_process_signal_batch_success_and_error() {
        let mut app = MockApp::default();
        let batch = Signal::Batch(vec![
            Signal::msg(MockMsg::Increment),
            Signal::msg(MockMsg::Increment),
        ]);
        let _ = process_signal(&mut app, batch).unwrap();
        assert_eq!(app.counter, 2);

        let mut app_err = MockApp::default();
        let batch_err = Signal::Batch(vec![
            Signal::msg(MockMsg::Fail("err1".to_owned())),
            Signal::msg(MockMsg::Increment),
            Signal::msg(MockMsg::Fail("err2".to_owned())),
        ]);
        let err = process_signal(&mut app_err, batch_err).unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("Multiple Errors Occurred"));
        assert!(err_str.contains("err1"));
        assert!(err_str.contains("err2"));
        assert_eq!(app_err.counter, 1);
    }

    #[test]
    fn test_process_signal_sequence_success_and_short_circuit() {
        let mut app = MockApp::default();
        let seq = Signal::Sequence(vec![
            Signal::msg(MockMsg::Increment),
            Signal::msg(MockMsg::Increment),
        ]);
        let _ = process_signal(&mut app, seq).unwrap();
        assert_eq!(app.counter, 2);

        let mut app_err = MockApp::default();
        let seq_err = Signal::Sequence(vec![
            Signal::msg(MockMsg::Fail("stop".to_owned())),
            Signal::msg(MockMsg::Increment),
        ]);
        let err = process_signal(&mut app_err, seq_err).unwrap_err();
        assert!(err.to_string().contains("stop"));
        assert_eq!(app_err.counter, 0);
    }

    #[test]
    fn test_process_signal_on_error_handling() {
        let mut app = MockApp::default();
        let on_err = Signal::msg(MockMsg::Fail("caught".to_owned())).on_error(MockMsg::FallbackHandled);
        let _ = process_signal(&mut app, on_err).unwrap();
        assert_eq!(app.counter, 100);

        let mut app_ignored = MockApp::default();
        let ignored = Signal::msg(MockMsg::Fail("ignored".to_owned())).ignore_error();
        let _ = process_signal(&mut app_ignored, ignored).unwrap();
        assert_eq!(app_ignored.counter, 0);

        let mut app_ok = MockApp::default();
        let ok_sig = Signal::msg(MockMsg::Increment).on_error(MockMsg::FallbackHandled);
        let _ = process_signal(&mut app_ok, ok_sig).unwrap();
        assert_eq!(app_ok.counter, 1);
    }

    #[test]
    fn test_handle_update_success_and_error() {
        let mut app = MockApp::default();
        let _task = handle_update(&mut app, MockMsg::Increment);
        assert_eq!(app.counter, 1);
        assert!(app.errors.is_empty());

        let _task_err = handle_update(&mut app, MockMsg::Fail("explosion".to_owned()));
        assert_eq!(app.errors.len(), 1);
        assert!(app.errors.first().is_some_and(|e| e.contains("explosion")));
    }

    #[derive(Debug)]
    struct MinimalApp;

    impl App for MinimalApp {
        type Message = ();

        fn boot() -> (Self, Option<Task<Self::Message>>) {
            (Self, None)
        }

        fn config<P>(
            app: Application<P>,
        ) -> Application<impl Program<State = Self, Message = Self::Message, Theme = Theme>>
        where
            P: Program<State = Self, Message = Self::Message, Theme = Theme>,
        {
            app
        }

        fn view(&self) -> Element<'_, Self::Message> {
            text("minimal").into()
        }

        fn update(&mut self, _message: Self::Message) -> anyhow::Result<Signal<Self::Message, ()>> {
            Signal::done()
        }
    }

    #[test]
    fn test_default_on_error_no_panic() {
        let mut minimal = MinimalApp;
        let err = anyhow::anyhow!("test error");
        minimal.on_error(&err);
    }

    #[cfg(feature = "logger")]
    mod logger_tests {
        use super::*;
        use std::sync::{Mutex, OnceLock};

        static LOG_EVENTS: OnceLock<Mutex<Vec<(log::Level, String)>>> = OnceLock::new();

        struct TestLogger;

        impl log::Log for TestLogger {
            fn enabled(&self, _metadata: &log::Metadata) -> bool {
                true
            }

            fn log(&self, record: &log::Record) {
                if let Some(events) = LOG_EVENTS.get()
                    && let Ok(mut lock) = events.lock()
                {
                    lock.push((record.level(), record.args().to_string()));
                }
            }

            fn flush(&self) {}
        }

        static TEST_LOGGER: TestLogger = TestLogger;

        fn init_test_logger() {
            LOG_EVENTS.get_or_init(|| Mutex::new(Vec::new()));
            let _ = log::set_logger(&TEST_LOGGER);
            log::set_max_level(log::LevelFilter::Trace);
        }

        #[test]
        fn test_logging_emissions() {
            init_test_logger();
            if let Some(events) = LOG_EVENTS.get() {
                events.lock().unwrap().clear();
            }

            let mut app = MockApp::default();

            // process_message emits debug log
            let _ = process_message(&mut app, MockMsg::Increment);

            // process_signal on_error emits debug log
            let on_err =
                Signal::msg(MockMsg::Fail("graceful err".to_owned())).on_error(MockMsg::FallbackHandled);
            let _ = process_signal(&mut app, on_err);

            // handle_update on error emits error log
            let _ = handle_update(&mut app, MockMsg::Fail("fatal err".to_owned()));

            let recorded = LOG_EVENTS
                .get()
                .and_then(|events| events.lock().ok().map(|l| l.clone()))
                .unwrap_or_default();

            assert!(
                recorded
                    .iter()
                    .any(|(lvl, msg)| *lvl == log::Level::Debug && msg.contains("Update: Increment")),
                "Expected debug log for update, got: {recorded:?}"
            );
            assert!(
                recorded.iter().any(|(lvl, msg)| *lvl == log::Level::Debug
                    && msg.contains("Gracefully Caught Error: graceful err")),
                "Expected debug log for graceful error, got: {recorded:?}"
            );
            assert!(
                recorded
                    .iter()
                    .any(|(lvl, msg)| *lvl == log::Level::Error && msg.contains("Error: fatal err")),
                "Expected error log for handle_update error, got: {recorded:?}"
            );
        }
    }
}
