fn main() {
    let content = topcoat::view! {
        <html>
            <head>
                <title>"hello world"</title>
            </head>
            <body>
                for name in ["carl", "julien"] {
                    if name.len() < 5 {
                        let name = name.to_uppercase();
                        <div>
                            "hello " (name)
                            <input />
                        </div>
                    } else {
                        <br>
                        "im " (name)
                    }
                }
            </body>
        </html>
    };

    println!("{}", content);
}
