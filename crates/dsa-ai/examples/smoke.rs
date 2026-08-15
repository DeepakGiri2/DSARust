//! Manual end-to-end check against a real Ollama server:
//!     cargo run -p dsa-ai --example smoke
//! Skipped in CI (needs a local server), which is why it is an example and
//! not a #[test].

fn main() {
    let url = std::env::var("DSA_OLLAMA").unwrap_or_else(|_| dsa_ai::DEFAULT_OLLAMA_URL.into());
    let models = match dsa_ai::list_models(&url) {
        Ok(m) => m,
        Err(e) => {
            println!("offline: {e}");
            return;
        }
    };
    println!("{} model(s); using {}", models.len(), models[0]);

    let mut stream = dsa_ai::chat_stream(dsa_ai::ChatOptions {
        url,
        model: models[0].clone(),
        messages: vec![
            dsa_ai::ChatMessage::system("Reply with exactly one short sentence."),
            dsa_ai::ChatMessage::user("Say hello."),
        ],
        temperature: 0.2,
    });

    let started = std::time::Instant::now();
    let mut content = String::new();
    let mut thinking = 0usize;
    while !stream.finished() && started.elapsed().as_secs() < 90 {
        for ev in stream.poll() {
            match ev {
                dsa_ai::Event::Token(dsa_ai::Channel::Content, t) => content.push_str(&t),
                dsa_ai::Event::Token(dsa_ai::Channel::Thinking, t) => thinking += t.len(),
                dsa_ai::Event::Error(e) => println!("error: {e}"),
                dsa_ai::Event::Done => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    println!("thinking chars: {thinking}");
    println!("reply: {}", content.trim());
    println!("streamed in {:.1}s", started.elapsed().as_secs_f32());
}
