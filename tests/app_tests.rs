use iced::widget::text;
use iced::{Application, Element, Program, Task, Theme, application};
use iced_tea::{App, Signal};

#[derive(Debug, Default)]
struct TestApp {
    counter: i32,
    errors_logged: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppMsg {
    Increment,
    Fail(String),
    BatchTrigger,
    SequenceTrigger,
    ChainTrigger,
    FallbackHandled,
}

impl App for TestApp {
    type Message = AppMsg;

    fn boot() -> (Self, Option<Task<Self::Message>>) {
        (Self::default(), None)
    }

    fn config<P>(
        app: Application<P>,
    ) -> Application<impl Program<State = Self, Message = Self::Message, Theme = Theme>>
    where
        P: Program<State = Self, Message = Self::Message, Theme = Theme>,
    {
        app.title("Test App")
    }

    fn view(&self) -> Element<'_, Self::Message> {
        text("test").into()
    }

    fn update(&mut self, message: Self::Message) -> anyhow::Result<Signal<Self::Message, ()>> {
        match message {
            AppMsg::Increment => {
                self.counter += 1;
                Signal::done()
            }
            AppMsg::Fail(err) => Err(anyhow::anyhow!(err)),
            AppMsg::BatchTrigger => Ok(Signal::Batch(vec![
                Signal::msg(AppMsg::Increment),
                Signal::msg(AppMsg::Increment),
            ])),
            AppMsg::SequenceTrigger => Ok(Signal::Sequence(vec![
                Signal::msg(AppMsg::Increment),
                Signal::msg(AppMsg::Increment),
            ])),
            AppMsg::ChainTrigger => Ok(Signal::msg(AppMsg::Increment)),
            AppMsg::FallbackHandled => {
                self.counter += 100;
                Signal::done()
            }
        }
    }

    fn on_error(&mut self, error: &anyhow::Error) {
        self.errors_logged.push(format!("{error:#}"));
    }
}

#[derive(Debug, Default)]
struct MinimalApp {
    state: i32,
}

impl App for MinimalApp {
    type Message = ();

    fn boot() -> (Self, Option<Task<Self::Message>>) {
        (Self::default(), None)
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
        self.state += 1;
        Signal::done()
    }
}

#[test]
fn test_boot_and_view() {
    let (app, task) = TestApp::boot();
    assert_eq!(app.counter, 0);
    assert!(task.is_none());
    let _view = app.view();
}

#[test]
fn test_update_transitions() {
    let mut app = TestApp::default();
    let res = app.update(AppMsg::Increment).unwrap();
    assert_eq!(app.counter, 1);
    assert!(res.is_done());

    let fail_res = app.update(AppMsg::Fail("err".to_owned()));
    let _err = fail_res.unwrap_err();

    let batch_res = app.update(AppMsg::BatchTrigger).unwrap();
    assert!(matches!(batch_res, Signal::Batch(_)));

    let seq_res = app.update(AppMsg::SequenceTrigger).unwrap();
    assert!(matches!(seq_res, Signal::Sequence(_)));

    let chain_res = app.update(AppMsg::ChainTrigger).unwrap();
    assert!(matches!(chain_res, Signal::Message(AppMsg::Increment)));

    let fallback_res = app.update(AppMsg::FallbackHandled).unwrap();
    assert_eq!(app.counter, 101);
    assert!(fallback_res.is_done());
}

#[test]
fn test_config_builder() {
    let app = application(
        || {
            let (app, maybe_task) = TestApp::boot();
            (app, maybe_task.unwrap_or_else(Task::none))
        },
        |_app: &mut TestApp, _msg: AppMsg| Task::none(),
        TestApp::view,
    );
    let _configured = TestApp::config(app);
}

#[test]
fn test_on_error_handler() {
    let mut app = TestApp::default();
    let err = anyhow::anyhow!("custom error");
    app.on_error(&err);
    assert_eq!(app.errors_logged.len(), 1);
    assert_eq!(
        app.errors_logged.first().map(String::as_str),
        Some("custom error")
    );
}

#[test]
fn test_minimal_app_defaults() {
    let (mut app, task) = MinimalApp::boot();
    assert_eq!(app.state, 0);
    assert!(task.is_none());
    let _ = app.view();

    let sig = app.update(()).unwrap();
    assert_eq!(app.state, 1);
    assert!(sig.is_done());

    let err = anyhow::anyhow!("silent failure");
    app.on_error(&err); // default on_error should not panic
}
