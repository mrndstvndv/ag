//! ag — CLI chat client against the OpenCode Zen gateway. Streams with --stream.

mod zen;

use std::error::Error;
use std::io::Write;

use futures::StreamExt;

use zen::{chat, chat_stream, ChatMessage, ChatRequest, ChatStreamEvent, MODELS};

const DEFAULT_MODEL: &str = MODELS[0]; // deepseek-v4-flash-free: fast + stable

struct Args {
    prompt: String,
    model: String,
    stream: bool,
}

fn parse_args() -> Args {
    let mut model = DEFAULT_MODEL.to_string();
    let mut stream = false;
    let mut positionals: Vec<String> = vec![];

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stream" => stream = true,
            "--model" => model = args.next().unwrap_or_else(|| fail("--model needs a value")),
            "--help" => {
                println!("usage: ag [--stream] [--model MODEL] [PROMPT...]");
                std::process::exit(0);
            }
            _ if arg.starts_with("--") => fail(&format!("unknown flag: {arg}")),
            _ => positionals.push(arg),
        }
    }

    let prompt = if positionals.is_empty() {
        "Say hi in one sentence.".to_string()
    } else {
        positionals.join(" ")
    };

    Args { prompt, model, stream }
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args();

    let chat_req = ChatRequest::new(vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user(args.prompt),
    ]);

    if args.stream {
        let resp = chat_stream(&args.model, chat_req).await?;
        let mut stream = resp.stream;
        while let Some(event) = stream.next().await {
            match event {
                Ok(ChatStreamEvent::Chunk(chunk)) => {
                    print!("{}", chunk.content);
                    std::io::stdout().flush()?;
                }
                Ok(ChatStreamEvent::End(_)) => break,
                // Reasoning / tool-call chunks are not printed.
                Ok(_) => {}
                Err(err) => return Err(err.into()),
            }
        }
        println!();
    } else {
        let res = chat(&args.model, chat_req).await?;
        println!("{}", res.first_text().unwrap_or("NO ANSWER"));
    }

    Ok(())
}