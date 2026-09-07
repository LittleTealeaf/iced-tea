use iced::Task;
use iced_tea::Signal;

#[derive(Debug, PartialEq, Eq)]
enum Msg {
    Tick,
    Increment(u32),
}

#[derive(Debug, PartialEq, Eq)]
enum Out {
    Closed,
    Notified(String),
}

#[test]
fn test_default_is_done() {
    let sig: Signal<Msg, Out> = Signal::default();
    assert!(sig.is_done());
    assert!(matches!(sig, Signal::Done));
}

#[test]
fn test_msg_constructor() {
    let sig: Signal<Msg, Out> = Signal::msg(Msg::Tick);
    assert!(!sig.is_done());
    assert!(matches!(sig, Signal::Message(Msg::Tick)));

    let sig2: Signal<Msg, Out> = Signal::msg(Msg::Increment(42));
    assert!(matches!(sig2, Signal::Message(Msg::Increment(42))));
}

#[test]
fn test_out_constructor() {
    let sig: Signal<Msg, Out> = Signal::out(Out::Closed);
    assert!(!sig.is_done());
    assert!(matches!(sig, Signal::Out(Out::Closed)));

    let sig2: Signal<Msg, Out> = Signal::out(Out::Notified("hello".to_owned()));
    assert!(matches!(sig2, Signal::Out(Out::Notified(s)) if s == "hello"));
}

#[test]
fn test_ok_and_done_result_helpers() {
    let res: anyhow::Result<Signal<Msg, Out>> = Signal::done();
    assert!(res.is_ok());
    let sig = res.unwrap();
    assert!(sig.is_done());
    assert!(matches!(sig, Signal::Done));

    let res_ok: anyhow::Result<Signal<Msg, Out>> = Signal::msg(Msg::Tick).ok();
    assert!(res_ok.is_ok());
    let sig_ok = res_ok.unwrap();
    assert!(matches!(sig_ok, Signal::Message(Msg::Tick)));
}

#[test]
fn test_is_done_predicate() {
    let done_sig: Signal<Msg, Out> = Signal::Done;
    assert!(done_sig.is_done());

    let msg_sig: Signal<Msg, Out> = Signal::msg(Msg::Tick);
    assert!(!msg_sig.is_done());

    let out_sig: Signal<Msg, Out> = Signal::out(Out::Closed);
    assert!(!out_sig.is_done());

    let task_sig: Signal<Msg, Out> = Signal::task(Task::none());
    assert!(!task_sig.is_done());
}

#[test]
fn test_task_constructors() {
    let task_sig: Signal<Msg, Out> = Signal::task(Task::none());
    assert!(matches!(task_sig, Signal::Task(_)));

    let from_sig: Signal<Msg, Out> = Signal::from(Task::none());
    assert!(matches!(from_sig, Signal::Task(_)));

    let delayed_sig: Signal<Msg, Out> = Signal::delayed(Msg::Tick);
    assert!(matches!(delayed_sig, Signal::Task(_)));
}

#[test]
fn test_perform_and_future() {
    let perform_sig: Signal<Msg, Out> = Signal::perform(async { 10 }, Msg::Increment);
    assert!(matches!(perform_sig, Signal::Task(_)));

    let future_sig: Signal<Msg, Out> = Signal::future(async { Msg::Tick });
    assert!(matches!(future_sig, Signal::Task(_)));
}

#[test]
fn test_error_handlers() {
    let on_err: Signal<Msg, Out> = Signal::msg(Msg::Tick).on_error(Msg::Increment(0));
    assert!(
        matches!(&on_err, Signal::OnError(inner, Some(Msg::Increment(0))) if matches!(**inner, Signal::Message(Msg::Tick)))
    );

    let ignore_err: Signal<Msg, Out> = Signal::msg(Msg::Tick).ignore_error();
    assert!(
        matches!(&ignore_err, Signal::OnError(inner, None) if matches!(**inner, Signal::Message(Msg::Tick)))
    );

    let maybe_some: Signal<Msg, Out> = Signal::msg(Msg::Tick).maybe_on_error(Some(Msg::Increment(1)));
    assert!(
        matches!(&maybe_some, Signal::OnError(inner, Some(Msg::Increment(1))) if matches!(**inner, Signal::Message(Msg::Tick)))
    );

    let maybe_none: Signal<Msg, Out> = Signal::msg(Msg::Tick).maybe_on_error::<Msg>(None);
    assert!(
        matches!(&maybe_none, Signal::OnError(inner, None) if matches!(**inner, Signal::Message(Msg::Tick)))
    );
}

#[test]
fn test_batch_combinator() {
    let empty: Signal<Msg, Out> = Signal::batch(Vec::new());
    assert!(matches!(empty, Signal::Done));

    let single: Signal<Msg, Out> = Signal::batch([Signal::msg(Msg::Tick)]);
    assert!(matches!(single, Signal::Message(Msg::Tick)));

    let multi: Signal<Msg, Out> = Signal::batch([Signal::msg(Msg::Tick), Signal::out(Out::Closed)]);
    if let Signal::Batch(signals) = multi {
        assert_eq!(signals.len(), 2);
        assert!(matches!(signals.first(), Some(Signal::Message(Msg::Tick))));
        assert!(matches!(signals.get(1), Some(Signal::Out(Out::Closed))));
    } else {
        panic!("expected Signal::Batch");
    }

    let nested: Signal<Msg, Out> = Signal::batch([
        Signal::Batch(vec![Signal::msg(Msg::Tick), Signal::msg(Msg::Increment(1))]),
        Signal::out(Out::Closed),
    ]);
    if let Signal::Batch(signals) = nested {
        assert_eq!(signals.len(), 3);
    } else {
        panic!("expected flattened Signal::Batch");
    }

    let with_done: Signal<Msg, Out> = Signal::batch([Signal::Done, Signal::msg(Msg::Tick), Signal::Done]);
    assert!(matches!(with_done, Signal::Message(Msg::Tick)));

    let all_done: Signal<Msg, Out> = Signal::batch([Signal::Done, Signal::Done]);
    assert!(matches!(all_done, Signal::Done));
}

#[test]
fn test_sequence_combinator() {
    let empty: Signal<Msg, Out> = Signal::sequence(Vec::new());
    assert!(matches!(empty, Signal::Done));

    let single: Signal<Msg, Out> = Signal::sequence([Signal::msg(Msg::Tick)]);
    assert!(matches!(single, Signal::Message(Msg::Tick)));

    let multi: Signal<Msg, Out> = Signal::sequence([Signal::msg(Msg::Tick), Signal::out(Out::Closed)]);
    if let Signal::Sequence(signals) = multi {
        assert_eq!(signals.len(), 2);
        assert!(matches!(signals.first(), Some(Signal::Message(Msg::Tick))));
        assert!(matches!(signals.get(1), Some(Signal::Out(Out::Closed))));
    } else {
        panic!("expected Signal::Sequence");
    }

    let nested: Signal<Msg, Out> = Signal::sequence([
        Signal::Sequence(vec![Signal::msg(Msg::Tick), Signal::msg(Msg::Increment(1))]),
        Signal::out(Out::Closed),
    ]);
    if let Signal::Sequence(signals) = nested {
        assert_eq!(signals.len(), 3);
    } else {
        panic!("expected flattened Signal::Sequence");
    }

    let with_done: Signal<Msg, Out> = Signal::sequence([Signal::Done, Signal::msg(Msg::Tick), Signal::Done]);
    assert!(matches!(with_done, Signal::Message(Msg::Tick)));

    let all_done: Signal<Msg, Out> = Signal::sequence([Signal::Done, Signal::Done]);
    assert!(matches!(all_done, Signal::Done));
}

#[test]
fn test_chain_combinator() {
    let a: Signal<Msg, Out> = Signal::msg(Msg::Tick);
    let b: Signal<Msg, Out> = Signal::out(Out::Closed);

    let done_left: Signal<Msg, Out> = Signal::Done.chain(Signal::msg(Msg::Tick));
    assert!(matches!(done_left, Signal::Message(Msg::Tick)));

    let done_right: Signal<Msg, Out> = Signal::msg(Msg::Tick).chain(Signal::Done);
    assert!(matches!(done_right, Signal::Message(Msg::Tick)));

    let two: Signal<Msg, Out> = a.chain(b);
    if let Signal::Sequence(signals) = &two {
        assert_eq!(signals.len(), 2);
    } else {
        panic!("expected Signal::Sequence");
    }

    let prepend: Signal<Msg, Out> = Signal::msg(Msg::Increment(1)).chain(two);
    if let Signal::Sequence(signals) = &prepend {
        assert_eq!(signals.len(), 3);
        assert!(matches!(
            signals.first(),
            Some(Signal::Message(Msg::Increment(1)))
        ));
    } else {
        panic!("expected Signal::Sequence with prepended item");
    }

    let append: Signal<Msg, Out> = prepend.chain(Signal::msg(Msg::Increment(2)));
    if let Signal::Sequence(signals) = &append {
        assert_eq!(signals.len(), 4);
        assert!(matches!(signals.get(3), Some(Signal::Message(Msg::Increment(2)))));
    } else {
        panic!("expected Signal::Sequence with appended item");
    }

    let seq1: Signal<Msg, Out> = Signal::Sequence(vec![Signal::msg(Msg::Tick), Signal::out(Out::Closed)]);
    let seq2: Signal<Msg, Out> = Signal::Sequence(vec![
        Signal::msg(Msg::Increment(1)),
        Signal::msg(Msg::Increment(2)),
    ]);
    let merged_seq: Signal<Msg, Out> = seq1.chain(seq2);
    if let Signal::Sequence(signals) = merged_seq {
        assert_eq!(signals.len(), 4);
    } else {
        panic!("expected Signal::Sequence with merged sequences");
    }
}

#[test]
fn test_merge_combinator() {
    let done_left: Signal<Msg, Out> = Signal::Done.merge(Signal::msg(Msg::Tick));
    assert!(matches!(done_left, Signal::Message(Msg::Tick)));

    let done_right: Signal<Msg, Out> = Signal::msg(Msg::Tick).merge(Signal::Done);
    assert!(matches!(done_right, Signal::Message(Msg::Tick)));

    let a: Signal<Msg, Out> = Signal::msg(Msg::Tick);
    let b: Signal<Msg, Out> = Signal::out(Out::Closed);
    let two: Signal<Msg, Out> = a.merge(b);
    if let Signal::Batch(signals) = &two {
        assert_eq!(signals.len(), 2);
    } else {
        panic!("expected Signal::Batch");
    }

    let add_to_batch: Signal<Msg, Out> = two.merge(Signal::msg(Msg::Increment(1)));
    if let Signal::Batch(signals) = &add_to_batch {
        assert_eq!(signals.len(), 3);
    } else {
        panic!("expected Signal::Batch with 3 items");
    }

    let item_merge_batch: Signal<Msg, Out> =
        Signal::msg(Msg::Increment(2)).merge(Signal::Batch(vec![Signal::msg(Msg::Tick)]));
    if let Signal::Batch(signals) = item_merge_batch {
        assert_eq!(signals.len(), 2);
    } else {
        panic!("expected Signal::Batch with 2 items");
    }

    let batch1: Signal<Msg, Out> = Signal::Batch(vec![Signal::msg(Msg::Tick), Signal::out(Out::Closed)]);
    let batch2: Signal<Msg, Out> = Signal::Batch(vec![
        Signal::msg(Msg::Increment(1)),
        Signal::msg(Msg::Increment(2)),
    ]);
    let merged_batches: Signal<Msg, Out> = batch1.merge(batch2);
    if let Signal::Batch(signals) = merged_batches {
        assert_eq!(signals.len(), 4);
    } else {
        panic!("expected Signal::Batch with 4 items");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParentMsg {
    Child(Msg),
    Custom(String),
}

impl From<Msg> for ParentMsg {
    fn from(msg: Msg) -> Self {
        Self::Child(msg)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParentOut {
    Forwarded(String),
}

#[test]
fn test_map_done() {
    let sig: Signal<Msg, Out> = Signal::Done;
    let mapped: Signal<ParentMsg, ParentOut> = sig.map(|_out| Ok(Signal::Done)).unwrap();
    assert!(matches!(mapped, Signal::Done));
}

#[test]
fn test_map_message() {
    let sig: Signal<Msg, Out> = Signal::msg(Msg::Tick);
    let mapped: Signal<ParentMsg, ParentOut> = sig.map(|_out| Ok(Signal::Done)).unwrap();
    assert!(matches!(mapped, Signal::Message(ParentMsg::Child(Msg::Tick))));
}

#[test]
fn test_map_out() {
    let sig: Signal<Msg, Out> = Signal::out(Out::Closed);
    let mapped: Signal<ParentMsg, ParentOut> = sig
        .map(|out| match out {
            Out::Closed => Ok(Signal::msg(ParentMsg::Custom("closed".to_owned()))),
            Out::Notified(_) => Ok(Signal::Done),
        })
        .unwrap();
    assert!(matches!(mapped, Signal::Message(ParentMsg::Custom(s)) if s == "closed"));
}

#[test]
fn test_map_batch_and_sequence() {
    let batch: Signal<Msg, Out> = Signal::Batch(vec![Signal::msg(Msg::Tick), Signal::out(Out::Closed)]);
    let mapped_batch: Signal<ParentMsg, ParentOut> = batch
        .map(|out| match out {
            Out::Closed => Ok(Signal::out(ParentOut::Forwarded("closed".to_owned()))),
            Out::Notified(_) => Ok(Signal::Done),
        })
        .unwrap();
    if let Signal::Batch(signals) = mapped_batch {
        assert_eq!(signals.len(), 2);
        assert!(matches!(
            signals.first(),
            Some(Signal::Message(ParentMsg::Child(Msg::Tick)))
        ));
        assert!(matches!(
            signals.get(1),
            Some(Signal::Out(ParentOut::Forwarded(s))) if s == "closed"
        ));
    } else {
        panic!("expected Signal::Batch");
    }

    let seq: Signal<Msg, Out> = Signal::Sequence(vec![Signal::msg(Msg::Tick), Signal::out(Out::Closed)]);
    let mapped_seq: Signal<ParentMsg, ParentOut> = seq
        .map(|out| match out {
            Out::Closed => Ok(Signal::out(ParentOut::Forwarded("closed".to_owned()))),
            Out::Notified(_) => Ok(Signal::Done),
        })
        .unwrap();
    if let Signal::Sequence(signals) = mapped_seq {
        assert_eq!(signals.len(), 2);
        assert!(matches!(
            signals.first(),
            Some(Signal::Message(ParentMsg::Child(Msg::Tick)))
        ));
        assert!(matches!(
            signals.get(1),
            Some(Signal::Out(ParentOut::Forwarded(s))) if s == "closed"
        ));
    } else {
        panic!("expected Signal::Sequence");
    }
}

#[test]
fn test_map_on_error() {
    let sig: Signal<Msg, Out> = Signal::msg(Msg::Tick).on_error(Msg::Increment(5));
    let mapped: Signal<ParentMsg, ParentOut> = sig.map(|_out| Ok(Signal::Done)).unwrap();
    if let Signal::OnError(inner, fallback) = mapped {
        assert!(matches!(*inner, Signal::Message(ParentMsg::Child(Msg::Tick))));
        assert_eq!(fallback, Some(ParentMsg::Child(Msg::Increment(5))));
    } else {
        panic!("expected Signal::OnError");
    }
}

#[test]
fn test_map_short_circuit_err() {
    let batch: Signal<Msg, Out> = Signal::Batch(vec![Signal::out(Out::Closed), Signal::msg(Msg::Tick)]);
    let res_batch: anyhow::Result<Signal<ParentMsg, ParentOut>> =
        batch.map(|_out| Err(anyhow::anyhow!("batch map failed")));
    _ = res_batch.unwrap_err();

    let seq: Signal<Msg, Out> = Signal::Sequence(vec![Signal::out(Out::Closed), Signal::msg(Msg::Tick)]);
    let res_seq: anyhow::Result<Signal<ParentMsg, ParentOut>> =
        seq.map(|_out| Err(anyhow::anyhow!("seq map failed")));
    _ = res_seq.unwrap_err();
}

#[test]
fn test_map_empty() {
    let sig_msg: Signal<Msg, ()> = Signal::msg(Msg::Tick);
    let res: Signal<ParentMsg, ParentOut> = sig_msg.map_empty().unwrap();
    assert!(matches!(res, Signal::Message(ParentMsg::Child(Msg::Tick))));

    let sig_done: Signal<Msg, ()> = Signal::Done;
    let res_done: Signal<ParentMsg, ParentOut> = sig_done.map_empty().unwrap();
    assert!(matches!(res_done, Signal::Done));
}
