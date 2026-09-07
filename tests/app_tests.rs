use iced::widget::text;
use iced::{Element, Task};
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

    fn title(&self) -> String {
        "Test App".to_owned()
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

#[test]
fn test_boot_title_view_defaults() {
    let (app, task) = TestApp::boot();
    assert_eq!(app.counter, 0);
    assert!(task.is_none());
    assert_eq!(app.title(), "Test App");
    let _view = app.view();
    let _theme = app.theme();
    let _sub = app.subscription();
    assert!((app.scale_factor() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_process_message_modifies_state() {
    let mut app = TestApp::default();
    let res = app.process_message(AppMsg::Increment);
    _ = res.unwrap();
    assert_eq!(app.counter, 1);
}

#[test]
fn test_process_signal_recursively_executes_message() {
    let mut app = TestApp::default();
    let res = app.process_signal(Signal::msg(AppMsg::Increment));
    _ = res.unwrap();
    assert_eq!(app.counter, 1);

    let res2 = app.process_effect(Signal::msg(AppMsg::Increment));
    _ = res2.unwrap();
    assert_eq!(app.counter, 2);

    let _ = app.process_signal(Signal::Done).unwrap();
    let _ = app.process_signal(Signal::Out(())).unwrap();
    let _ = app.process_signal(Signal::Task(Task::none())).unwrap();
}

#[test]
fn test_process_signal_batch_and_error_aggregation() {
    let mut app = TestApp::default();
    let batch = Signal::Batch(vec![
        Signal::msg(AppMsg::Increment),
        Signal::msg(AppMsg::Increment),
    ]);
    let res = app.process_signal(batch);
    _ = res.unwrap();
    assert_eq!(app.counter, 2);

    let mut app_err = TestApp::default();
    let batch_err = Signal::Batch(vec![
        Signal::msg(AppMsg::Fail("err1".to_owned())),
        Signal::msg(AppMsg::Increment),
        Signal::msg(AppMsg::Fail("err2".to_owned())),
    ]);
    let err = app_err.process_signal(batch_err).unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("Multiple Errors Occurred"));
    assert!(err_str.contains("err1"));
    assert!(err_str.contains("err2"));
    assert_eq!(app_err.counter, 1);
}

#[test]
fn test_process_signal_sequence_and_short_circuit() {
    let mut app = TestApp::default();
    let seq = Signal::Sequence(vec![
        Signal::msg(AppMsg::Increment),
        Signal::msg(AppMsg::Increment),
    ]);
    let res = app.process_signal(seq);
    _ = res.unwrap();
    assert_eq!(app.counter, 2);

    let mut app_err = TestApp::default();
    let seq_err = Signal::Sequence(vec![
        Signal::msg(AppMsg::Fail("stop".to_owned())),
        Signal::msg(AppMsg::Increment),
    ]);
    let err = app_err.process_signal(seq_err).unwrap_err();
    assert!(err.to_string().contains("stop"));
    assert_eq!(app_err.counter, 0);
}

#[test]
fn test_process_signal_on_error() {
    let mut app = TestApp::default();
    let on_err = Signal::msg(AppMsg::Fail("caught".to_owned())).on_error(AppMsg::FallbackHandled);
    let res = app.process_signal(on_err);
    _ = res.unwrap();
    assert_eq!(app.counter, 100);

    let mut app_ignored = TestApp::default();
    let ignored = Signal::msg(AppMsg::Fail("ignored".to_owned())).ignore_error();
    let res_ignored = app_ignored.process_signal(ignored);
    _ = res_ignored.unwrap();
    assert_eq!(app_ignored.counter, 0);

    let mut app_ok = TestApp::default();
    let ok_sig = Signal::msg(AppMsg::Increment).on_error(AppMsg::FallbackHandled);
    let res_ok = app_ok.process_signal(ok_sig);
    _ = res_ok.unwrap();
    assert_eq!(app_ok.counter, 1);
}

#[test]
fn test_handle_update_routes_to_on_error() {
    let mut app = TestApp::default();
    let _task = app.handle_update(AppMsg::Increment);
    assert_eq!(app.counter, 1);
    assert!(app.errors_logged.is_empty());

    let _task_err = app.handle_update(AppMsg::Fail("runtime explosion".to_owned()));
    assert_eq!(app.errors_logged.len(), 1);
    assert!(app.errors_logged.first().unwrap().contains("runtime explosion"));
}

#[test]
fn test_application_builder() {
    let _app_def = TestApp::application();
}
