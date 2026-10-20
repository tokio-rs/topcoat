use std::time::{Duration, SystemTime, UNIX_EPOCH};

use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{procedure, signal},
    view::{View, view},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let rendered_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    let flash = signal(cx, String::new);
    let moved = signal(cx, || 0.0);

    Ok(view! {
        // `@mount` runs in the browser after the runtime attaches to the
        // element: on page load, after navigation, and after a server render
        // replaces the element.
        <section
            @mount=$(|_e| {
                let _formatted = raw!(
                    "document.querySelectorAll('time[data-ms]').forEach((time) => {
                        time.textContent = new Date(Number(time.dataset.ms)).toLocaleString();
                    })",
                    (),
                );
            })
        >
            <h2>"Local time"</h2>
            <p>
                "Rendered at "
                <time data-ms=(rendered_at)>
                    (rendered_at)
                    " ms since the epoch"
                </time>
            </p>
        </section>

        // Shows a message saved before a reload, then forgets it.
        <section
            @mount=$(|_e| flash.set(
                    raw!(
                        "(() => {
                    const message = sessionStorage.getItem('flash') ?? '';
                    sessionStorage.removeItem('flash');
                    return cx.hydrate(message);
                })()",
                        String::new(),
                    ),
                ))
        >
            <h2>"Flash message"</h2>
            <p role="status" :hidden=$(flash.get().is_empty())>$(flash.get())</p>
            <button
                @click=$(|_e| raw!(
                        "(sessionStorage.setItem('flash', 'Saved before the reload'), location.reload())",
                        (),
                    ))
            >
                "Save and reload"
            </button>
        </section>

        // Resumes a multi-step operation that a reload interrupted. Progress
        // lives in session storage, so reloading mid-way picks it up again.
        <section
            @mount=$(async |_e| {
                let pending = raw!(
                    "cx.hydrate(sessionStorage.getItem('moved') !== null)",
                    false,
                );
                if pending {
                    moved.set(
                        raw!("cx.hydrate(Number(sessionStorage.getItem('moved')))", 0.0),
                    );
                    while moved.get() < 5.0 {
                        let next = move_one(moved.get()).await;
                        let _saved = raw!(
                            "sessionStorage.setItem('moved', String(${next}))",
                            (),
                        );
                        moved.set(next);
                    }
                    let _finished = raw!("sessionStorage.removeItem('moved')", ());
                }
            })
        >
            <h2>"Resumable operation"</h2>
            <p>
                "Moved "
                $(moved.get())
                " of 5 items."
            </p>
            <button
                @click=$(|_e| raw!(
                        "(sessionStorage.setItem('moved', '0'), location.reload())",
                        (),
                    ))
            >
                "Start, then reload mid-way"
            </button>
        </section>
    })
}

/// Moves one more item, slowly enough to interrupt with a reload.
// Validate arguments here in a real application, since the browser supplies them.
#[procedure]
pub async fn move_one(moved: f64) -> Result<f64> {
    tokio::time::sleep(Duration::from_millis(700)).await;
    Ok(moved + 1.0)
}
