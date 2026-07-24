use topcoat::{
    context::Cx,
    runtime::{Signal, SignalDeclaration},
    view::view,
};

fn r(v: topcoat::Result) -> String {
    v.unwrap().render(&Cx::default())
}

/// Extracts the `id` of every `::topcoat::signal({...})` declaration comment.
fn declared_ids(html: &str) -> Vec<String> {
    html.match_indices("::topcoat::signal(")
        .map(|(at, _)| {
            let payload = &html[at..];
            let key = "&quot;id&quot;:&quot;";
            let start = payload.find(key).expect("declaration payload has an id") + key.len();
            let end = payload[start..].find("&quot;").expect("id is terminated") + start;
            payload[start..end].to_string()
        })
        .collect()
}

/// Signals created per element of a runtime collection are declared to the
/// browser and captured by runtime expressions, like statement-declared ones.
#[tokio::test]
async fn runtime_created_signals_declare_and_capture() {
    let defaults: Vec<f64> = vec![2.0, 4.0, 1.0];
    let signals: Vec<Signal<f64>> = defaults.iter().map(|&v| Signal::new(v)).collect();

    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        for signal in &signals {
            (SignalDeclaration::new(signal))
        }

        for index in 0..defaults.len() {
            let each = &signals[index];
            <span>$(each.get())</span>
        }

        let first = &signals[0];
        let second = &signals[1];
        <p>$(first.get() + second.get())</p>
    });

    // one declaration per collection element, each with a distinct id
    let ids = declared_ids(&html);
    assert_eq!(ids.len(), 3);
    for (i, id) in ids.iter().enumerate() {
        assert!(!id.is_empty());
        assert!(!ids[..i].contains(id), "declaration ids are distinct");
    }

    // every declared signal is hydrated again by a capturing expression
    for id in &ids {
        assert!(
            html.matches(id.as_str()).count() >= 2,
            "signal {id} is captured by at least one expression"
        );
        let capture = format!("cx.hydrate({{&quot;t&quot;:&quot;Signal&quot;,&quot;id&quot;:&quot;{id}&quot;}})");
        assert!(html.contains(&capture), "signal {id} is hydrated by an expression");
    }

    // the initial render evaluated the expressions server-side; the values
    // sit between the expression markers
    for value in ["2", "4", "1", "6"] {
        let rendered = format!("-->{value}<!-- ::topcoat::expr::end -->");
        assert!(html.contains(&rendered), "server-rendered initial value {value}");
    }
}

/// The cross-item expression captures BOTH signals it reads, not just one.
#[tokio::test]
async fn cross_item_expression_captures_both_signals() {
    let signals: Vec<Signal<f64>> = vec![Signal::new(1.0), Signal::new(2.0)];

    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        for signal in &signals {
            (SignalDeclaration::new(signal))
        }

        let a = &signals[0];
        let b = &signals[1];
        <p>$(a.get() * b.get())</p>
    });

    let ids = declared_ids(&html);
    assert_eq!(ids.len(), 2);
    for id in &ids {
        assert!(
            html.matches(id.as_str()).count() >= 2,
            "signal {id} appears in the capturing expression"
        );
    }
    assert!(html.contains("-->2<!-- ::topcoat::expr::end -->"), "product evaluated server-side");
}
