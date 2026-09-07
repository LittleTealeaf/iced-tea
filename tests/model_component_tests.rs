use iced::Element;
use iced::widget::text;
use iced_tea::{Component, HandleMessage, Model, Signal};

// --- Child Model with OutMessage ---
#[derive(Debug, Default)]
struct Counter {
    value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CounterMsg {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CounterOut {
    ThresholdReached(i32),
}

impl Model for Counter {
    type Message = CounterMsg;
    type OutMessage = CounterOut;
    type Context<'a> = ();

    fn update<'a>(
        &'a mut self,
        message: Self::Message,
        _context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>> {
        match message {
            CounterMsg::Increment => {
                self.value += 1;
                if self.value >= 10 {
                    Signal::out(CounterOut::ThresholdReached(self.value)).ok()
                } else {
                    Signal::done()
                }
            }
            CounterMsg::Decrement => {
                self.value -= 1;
                Signal::done()
            }
        }
    }
}

// Parent types
#[derive(Debug, Clone, PartialEq, Eq)]
enum AppMsg {
    Counter(CounterMsg),
    Alert(String),
}

impl From<CounterMsg> for AppMsg {
    fn from(msg: CounterMsg) -> Self {
        Self::Counter(msg)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppOut {
    Logged(String),
}

#[test]
fn test_child_model_map_update() {
    let mut counter = Counter { value: 9 };

    let sig: Signal<AppMsg, AppOut> = counter
        .map_update(CounterMsg::Increment, (), |out| match out {
            CounterOut::ThresholdReached(val) => Signal::msg(AppMsg::Alert(format!("Reached {val}"))).ok(),
        })
        .unwrap();

    assert_eq!(counter.value, 10);
    assert!(matches!(sig, Signal::Message(AppMsg::Alert(ref s)) if s == "Reached 10"));

    // When no out message is emitted:
    let sig2: Signal<AppMsg, AppOut> = counter
        .map_update(CounterMsg::Decrement, (), |_out| Signal::done())
        .unwrap();
    assert_eq!(counter.value, 9);
    assert!(sig2.is_done());
}

// --- Void Child Model (OutMessage = ()) ---
#[derive(Debug, Default)]
struct Toggle {
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ToggleMsg {
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RootMsg {
    ChildToggle(ToggleMsg),
}

impl From<ToggleMsg> for RootMsg {
    fn from(msg: ToggleMsg) -> Self {
        Self::ChildToggle(msg)
    }
}

impl Model for Toggle {
    type Message = ToggleMsg;
    type OutMessage = ();
    type Context<'a> = ();

    fn update<'a>(
        &'a mut self,
        message: Self::Message,
        _context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>> {
        match message {
            ToggleMsg::Toggle => {
                self.enabled = !self.enabled;
                Signal::done()
            }
        }
    }
}

#[test]
fn test_void_child_model_empty_update() {
    let mut toggle = Toggle { enabled: false };

    let sig: Signal<RootMsg, AppOut> = toggle.empty_update(ToggleMsg::Toggle, ()).unwrap();
    assert!(toggle.enabled);
    assert!(sig.is_done());
}

// --- HandleMessage delegation ---
struct CustomDelegate;

impl Model for CustomDelegate {
    type Message = String;
    type OutMessage = ();
    type Context<'a> = ();

    fn update<'a>(
        &'a mut self,
        message: Self::Message,
        _context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>> {
        Signal::msg(message).ok()
    }
}

// Additional custom message handling
impl HandleMessage<u32> for CustomDelegate {
    fn handle_message<'a>(
        &'a mut self,
        message: u32,
        context: Self::Context<'a>,
    ) -> anyhow::Result<Signal<Self::Message, Self::OutMessage>> {
        self.handle_message(format!("{message}"), context)
    }
}

#[test]
fn test_handle_message() {
    let mut delegate = CustomDelegate;

    // Default HandleMessage<T::Message> delegating to update
    let sig = delegate.handle_message("direct".to_owned(), ()).unwrap();
    assert!(matches!(sig, Signal::Message(ref s) if s == "direct"));

    // Custom HandleMessage<u32>
    let sig2 = delegate.handle_message(123_u32, ()).unwrap();
    assert!(matches!(sig2, Signal::Message(ref s) if s == "123"));
}

// --- Component trait ---
impl Component for Counter {
    fn render<'a>(&'a self, _context: Self::Context<'a>) -> Element<'a, Self::Message> {
        text(format!("Count: {}", self.value)).into()
    }
}

#[test]
fn test_component_rendering() {
    let counter = Counter { value: 42 };

    // render
    let _el: Element<'_, CounterMsg> = counter.render(());

    // render_into
    let _el_parent: Element<'_, AppMsg> = counter.render_into(());

    // view
    let _view_el: Element<'_, CounterMsg> = counter.view(());

    // view_into
    let _view_el_parent: Element<'_, AppMsg> = counter.view_into(());
}
