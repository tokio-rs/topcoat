use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, procedure, signal},
    view::{View, view},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let input = signal(cx, String::new);

    Ok(view! {
        // Bind the input to the signal and save edits on the change event.
        <input :value=$(input.get()) @change=$(|e: Event| input.set(e.target.value))>

        // Send the input to the server procedure, then update the signal
        // with the response.
        <button
            @click=$(async |_e| {
                let server_response = print_on_server(input.get()).await;
                input.set(server_response);
            })
        >
            "Print on server"
        </button>
    })
}

// Validate arguments here in a real application, since the browser supplies them.
#[procedure]
pub async fn print_on_server(_cx: &Cx, input: String) -> Result<String> {
    println!("{input}");
    Ok(format!("message received: {input}"))
}
