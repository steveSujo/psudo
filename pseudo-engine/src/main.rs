use clap::Parser;
use colored::*;
use std::io::{self, Write};
use std::path::PathBuf;

use pseudo_engine::engine::Engine;
use pseudo_engine::pir_decompiler;
use pseudo_engine::sampler::{Sampler, SparseChannel};

// Separating the inference CLI from `psudo` lets me benchmark, chat, and run `--daemon`
// directly against a GGUF file without going through the compiler or sandbox harness.
#[derive(Parser, Debug)]
#[command(
    name = "pseudo-engine",
    author = "Cactus Compute / Psudo Ecosystem",
    version = "0.2.0",
    about = "Pure Rust Native Llama.cpp-like Inference Engine & CLI"
)]
struct Args {
    #[arg(short, long, default_value = "pseudo-33m-tq2_0.gguf")]
    model: PathBuf,

    #[arg(short, long, default_value = None)]
    tokenizer: Option<PathBuf>,

    #[arg(short, long, default_value = None)]
    prompt: Option<String>,

    #[arg(long, default_value_t = false)]
    chat: bool,

    #[arg(short, long, default_value_t = 1536)]
    ctx_size: usize,

    #[arg(long, default_value_t = 0.2)]
    temp: f32,

    #[arg(long, default_value_t = 0.95)]
    top_p: f32,

    #[arg(long, default_value_t = 40)]
    top_k: usize,

    #[arg(long, default_value_t = 1.1)]
    rep_penalty: f32,

    #[arg(long, default_value_t = false)]
    color: bool,

    #[arg(long, default_value_t = false)]
    benchmark: bool,

    #[arg(long, default_value_t = 4)]
    threads: usize,

    #[arg(long, default_value_t = false)]
    pir_mode: bool,

    #[arg(long, default_value_t = false)]
    raw_prompt: bool,

    #[arg(long, default_value_t = 512)]
    max_tokens: usize,

    #[arg(long, default_value_t = false)]
    mcp: bool,

    #[arg(long, default_value_t = false)]
    daemon: bool,

    #[arg(long, default_value_t = 11434)]
    port: u16,

    #[arg(long, default_value_t = 600)]
    idle_timeout: u64,
}

fn main() {
    let args = Args::parse();
    if args.daemon {
        let port = std::env::var("PSUDO_DAEMON_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(args.port);
        let resolved_model =
            psudo::mcp::resolve_model_path(Some(&args.model)).unwrap_or_else(|| args.model.clone());
        let engine = match Engine::new(&resolved_model, args.tokenizer.as_deref()) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("[pseudo-engine-daemon] Failed to initialize model: {}", err);
                std::process::exit(1);
            }
        };
        if let Err(err) =
            pseudo_engine::daemon::run_daemon(engine, port, args.idle_timeout, resolved_model)
        {
            eprintln!("[pseudo-engine-daemon] Daemon runtime error: {}", err);
            std::process::exit(1);
        }
        return;
    }

    if args.mcp {
        let resolved_model =
            psudo::mcp::resolve_model_path(Some(&args.model)).unwrap_or_else(|| args.model.clone());
        match Engine::new(&resolved_model, args.tokenizer.as_deref()) {
            Ok(mut engine) => {
                eprintln!("[psudo-mcp] Hot GGUF loaded: {}", resolved_model.display());
                let _ = psudo::mcp::serve_mcp_stdio(Some(resolved_model), |req| {
                    let channel = match req.channel.as_str() {
                        "clarify" => SparseChannel::Clarify,
                        "zettel" => SparseChannel::Zettel,
                        "code" => SparseChannel::Pir,
                        _ => SparseChannel::Pir,
                    };
                    let routed_prompt = match req.channel.as_str() {
                        "clarify" => format!("Prompt: {}\n<clarify>\n", req.prompt),
                        "fim" => req.prompt.clone(),
                        "zettel" => {
                            if req.prompt.is_empty() {
                                "<zettel>\n".to_string()
                            } else {
                                format!("{}\n<zettel>\n", req.prompt)
                            }
                        }
                        "code" => {
                            let p = req.prompt.trim();
                            if p.starts_with("f ") || p.starts_with("fn ") {
                                format!("{}\n<pir>\n", p)
                            } else {
                                format!("f {} (\n<pir>\n", p)
                            }
                        }
                        _ => req.prompt.clone(),
                    };

                    let constraint =
                        Box::new(engine.build_channel_constraint(channel, Some(&routed_prompt)));
                    let mut sampler = Sampler::new(
                        req.temp,
                        args.top_p,
                        args.top_k,
                        args.rep_penalty,
                        constraint,
                    );
                    let mut buf = String::new();
                    let metrics = engine
                        .generate(&routed_prompt, &mut sampler, req.max_tokens, |tok| {
                            buf.push_str(tok);
                        })
                        .map_err(|e| format!("Engine generation error: {}", e))?;

                    let output = if channel == SparseChannel::Pir {
                        if let Some(psudo_out) = pir_decompiler::decompile_pir_to_psudo(&buf) {
                            psudo_out
                        } else {
                            buf.trim().to_string()
                        }
                    } else {
                        buf.replace("<clarify>", "")
                            .replace("</talk>", "")
                            .replace("<zettel>", "")
                            .replace("</zettel>", "")
                            .trim()
                            .to_string()
                    };

                    Ok(psudo::mcp::McpGgufResponse {
                        output,
                        channel_used: req.channel.clone(),
                        ttft_ms: metrics.ttft_ms,
                        decode_tps: metrics.decode_tps,
                        confidence: metrics.confidence,
                        engine_mode: "hot_gguf_mmap".to_string(),
                    })
                });
            }
            Err(e) => {
                eprintln!(
                    "[psudo-mcp] GGUF load warning ({}); serving with fallback handler",
                    e
                );
                let _ =
                    psudo::mcp::serve_mcp_stdio(None, psudo::mcp::default_subprocess_gguf_handler);
            }
        }
        return;
    }

    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global();

    println!(
        "{}",
        "========================================================".bright_blue()
    );
    println!(
        "  {} - Pure Rust Native Inference Engine",
        "pseudo-engine".bright_green().bold()
    );
    println!("  Model: {}", args.model.display());
    println!(
        "{}",
        "========================================================".bright_blue()
    );

    let mut engine = match Engine::new(&args.model, args.tokenizer.as_deref()) {
        Ok(e) => {
            println!(
                "{} Successfully loaded GGUF model & vocabulary.",
                "✓".bright_green()
            );
            e
        }
        Err(err) => {
            eprintln!("{} Failed to initialize engine: {}", "✗".bright_red(), err);
            std::process::exit(1);
        }
    };

    if args.benchmark {
        println!("Running benchmark (warmup + 64 tokens)...");
        let warmup_prompt = "fn benchmark_func () [";
        let constraint = Box::new(engine.build_pir_constraint(true, Some(warmup_prompt)));
        let mut bench_sampler = Sampler::new(
            args.temp,
            args.top_p,
            args.top_k,
            args.rep_penalty,
            constraint,
        );
        let _ = engine.generate(warmup_prompt, &mut bench_sampler, 16, |_| {});

        let start = std::time::Instant::now();
        let metrics = engine
            .generate(warmup_prompt, &mut bench_sampler, 64, |_| {})
            .unwrap();
        let duration = start.elapsed();
        println!("=== Benchmark Results ===");
        println!("TTFT: {:.2} ms", metrics.ttft_ms);
        println!("Decode Speed: {:.2} tok/s", metrics.decode_tps);
        println!("Total Time for 64 tokens: {:.2} s", duration.as_secs_f64());
        return;
    }

    if args.chat {
        println!(
            "\n{} (Type 'exit' to quit)\n",
            "Interactive Chat Mode Active (Auto-Routing Code vs Ask)".bright_green()
        );
        let stdin = io::stdin();
        loop {
            print!("{}: ", "User".bright_cyan());
            io::stdout().flush().unwrap();
            let mut line = String::new();
            if stdin.read_line(&mut line).is_err() || line.trim() == "exit" {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (channel, routed_prompt) = pir_decompiler::route_prompt(trimmed, args.pir_mode);
            print!(
                "{}: {} ",
                "Engine".bright_green(),
                format!("[channel: {:?}, processing prefill...]", channel).dimmed()
            );
            io::stdout().flush().unwrap();
            print!("\x1b[2K\r{}: ", "Engine".bright_green());
            io::stdout().flush().unwrap();

            let mut local_sampler = Sampler::new(
                args.temp,
                args.top_p,
                args.top_k,
                args.rep_penalty,
                Box::new(engine.build_channel_constraint(channel, Some(&routed_prompt))),
            );

            let mut generated_buffer = String::new();
            match engine.generate(
                &routed_prompt,
                &mut local_sampler,
                args.max_tokens,
                |token| {
                    generated_buffer.push_str(token);
                    if args.color {
                        print!("{}", token.bright_yellow());
                    } else {
                        print!("{}", token);
                    }
                    io::stdout().flush().unwrap();
                },
            ) {
                Ok(metrics) => {
                    if channel == SparseChannel::Pir {
                        if let Some(psudo_out) =
                            pir_decompiler::decompile_pir_to_psudo(&generated_buffer)
                        {
                            println!(
                                "\n\n{}:\n{}",
                                ".psudo Decompiled View".bright_magenta(),
                                psudo_out.bright_white()
                            );
                        }
                    }
                    println!(
                        "\n{:.1} tps | TTFT: {:.1} ms | conf: {:.2}\n",
                        metrics.decode_tps, metrics.ttft_ms, metrics.confidence
                    );
                }
                Err(e) => {
                    println!("\nError generating response: {}\n", e);
                }
            }
        }
        return;
    }

    if let Some(prompt) = args.prompt {
        let (channel, routed_prompt) = if args.raw_prompt {
            let p_trim = prompt.trim();
            if args.pir_mode
                || p_trim.starts_with("f ")
                || p_trim.starts_with("fn ")
                || p_trim.starts_with("<pir>")
                || p_trim.ends_with("<pir>")
                || p_trim.contains("\n<pir>\n")
            {
                (SparseChannel::Pir, prompt.clone())
            } else {
                (SparseChannel::Unconstrained, prompt.clone())
            }
        } else {
            pir_decompiler::route_prompt(&prompt, args.pir_mode)
        };
        print!("\n{}: ", "Prompt".bright_cyan());
        println!("{}", prompt);
        print!("{}: ", "Output".bright_green());
        let _ = io::stdout().flush();

        let mut local_sampler = Sampler::new(
            args.temp,
            args.top_p,
            args.top_k,
            args.rep_penalty,
            Box::new(engine.build_channel_constraint(channel, Some(&routed_prompt))),
        );

        let mut generated_buffer = String::new();
        let metrics = engine
            .generate(
                &routed_prompt,
                &mut local_sampler,
                args.max_tokens,
                |token| {
                    generated_buffer.push_str(token);
                    if args.color {
                        print!("{}", token.bright_yellow());
                    } else {
                        print!("{}", token);
                    }
                    let _ = io::stdout().flush();
                },
            )
            .unwrap();

        if channel == SparseChannel::Pir {
            if let Some(psudo_out) = pir_decompiler::decompile_pir_to_psudo(&generated_buffer) {
                println!(
                    "\n\n{}:\n{}",
                    ".psudo Decompiled View".bright_magenta(),
                    psudo_out.bright_white()
                );
            }
        }

        println!(
            "\n\n{:.1} tps | TTFT: {:.1} ms | conf: {:.2}",
            metrics.decode_tps, metrics.ttft_ms, metrics.confidence
        );
    }
}
