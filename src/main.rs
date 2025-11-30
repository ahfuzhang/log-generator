use chrono::Local;
use clap::{Parser, ValueEnum};
use rand::{
    distributions::{Alphanumeric, DistString},
    seq::SliceRandom,
    Rng,
};
use reqwest::{blocking::Client, header::CONTENT_TYPE};
use serde_json::json;
use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

const HTTP_METHODS: &[&str] = &["POST", "GET", "HEAD", "PUT"];
const HTTP_VERSIONS: &[&str] = &["HTTP/1.1", "HTTP/2.0", "HTTP/3.0"];
const STATUS_CODES: &[u16] = &[
    200, 201, 204, 206, 301, 302, 304, 400, 401, 403, 404, 408, 418, 429, 500, 502, 503, 504,
];
const PATH_SEGMENTS: &[&str] = &[
    "game-api",
    "api",
    "v1",
    "v2",
    "profiles",
    "session",
    "items",
    "orders",
    "metrics",
    "events",
    "spin",
    "status",
];
const HOST_PREFIXES: &[&str] = &["api", "edge", "host", "svc", "gateway", "ingress"];

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate JSON logs continuously for load testing.")]
struct Args {
    #[arg(
        long = "sleep_ms",
        visible_alias = "sleep-ms",
        default_value = "0",
        help = "Sleep milliseconds after each batch is flushed."
    )]
    sleep_ms: u64,

    #[arg(
        long = "batch_bytes",
        visible_alias = "batch-bytes",
        default_value = "64k",
        value_parser = parse_byte_size,
        help = "Bytes to emit per batch (supports k/m/g suffixes)."
    )]
    batch_bytes: usize,

    #[arg(
        long = "output",
        value_enum,
        default_value_t = Output::Stdout,
        help = "Where to send logs: stdout or POST over http."
    )]
    output: Output,

    #[arg(
        long = "http.jsonline",
        visible_alias = "http-jsonline",
        required_if_eq("output", "http"),
        help = "HTTP endpoint to receive NDJSON batches when output=http."
    )]
    http_jsonline: Option<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
#[value(rename_all = "lower")]
enum Output {
    Stdout,
    Http,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    if args.batch_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "batch_bytes must be greater than 0",
        ));
    }

    match args.output {
        Output::Stdout => run_stdout(args.batch_bytes, args.sleep_ms),
        Output::Http => {
            let endpoint = args
                .http_jsonline
                .as_deref()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "http.jsonline is required when output=http"))?;
            run_http(endpoint, args.batch_bytes, args.sleep_ms)
        }
    }
}

fn run_stdout(batch_size: usize, sleep_ms: u64) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    run_with_sink(batch_size, sleep_ms, |chunk| {
        writer.write_all(chunk)?;
        writer.flush()
    })
}

fn run_http(endpoint: &str, batch_size: usize, sleep_ms: u64) -> io::Result<()> {
    let client = Client::new();
    run_with_sink(batch_size, sleep_ms, |chunk| {
        client
            .post(endpoint)
            .header(CONTENT_TYPE, "application/x-ndjson")
            .body(chunk.to_vec())
            .send()
            .and_then(|resp| resp.error_for_status())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(())
    })
}

fn run_with_sink<F>(batch_size: usize, sleep_ms: u64, mut sink: F) -> io::Result<()>
where
    F: FnMut(&[u8]) -> io::Result<()>,
{
    let mut rng = rand::thread_rng();
    let mut buffer = Vec::with_capacity(batch_size);
    let sleep_duration = (sleep_ms > 0).then(|| Duration::from_millis(sleep_ms));

    loop {
        buffer.clear();
        while buffer.len() < batch_size {
            let log_line = generate_log_line(&mut rng);
            let required = log_line.len() + 1; // +1 for trailing newline

            if required > batch_size && buffer.is_empty() {
                buffer.extend_from_slice(log_line.as_bytes());
                buffer.push(b'\n');
                break;
            }

            if buffer.len() + required > batch_size {
                break;
            }

            buffer.extend_from_slice(log_line.as_bytes());
            buffer.push(b'\n');
        }

        if !buffer.is_empty() {
            sink(&buffer)?;
        }

        if let Some(duration) = sleep_duration {
            thread::sleep(duration);
        }
    }
}

fn parse_byte_size(input: &str) -> Result<usize, String> {
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err("batch_bytes cannot be empty".into());
    }

    let (number_part, multiplier) = if let Some(num) = trimmed.strip_suffix('g') {
        (num, 1024usize.pow(3))
    } else if let Some(num) = trimmed.strip_suffix('m') {
        (num, 1024usize.pow(2))
    } else if let Some(num) = trimmed.strip_suffix('k') {
        (num, 1024usize)
    } else {
        (trimmed.as_str(), 1usize)
    };

    let value: usize = number_part
        .parse()
        .map_err(|_| format!("invalid number in batch_bytes: {input}"))?;

    value
        .checked_mul(multiplier)
        .ok_or_else(|| format!("batch_bytes too large: {input}"))
}

fn generate_log_line(rng: &mut impl Rng) -> String {
    let client_ip = random_ip(rng);
    let host = random_host(rng);
    let trace_id = random_trace_id(rng);
    let captured_headers = format!("{host} - {client_ip} -");

    let record = json!({
        "time": Local::now().format("%d/%b/%Y:%H:%M:%S%.3f").to_string(),
        "client_ip": client_ip,
        "bytes_read": rng.gen_range(200..5000).to_string(),
        "captured_request_headers": captured_headers,
        "http_method": HTTP_METHODS.choose(rng).unwrap(),
        "http_request_path": random_path(rng),
        "http_request_query_string": format!("?traceId={trace_id}"),
        "http_version": HTTP_VERSIONS.choose(rng).unwrap(),
        "server_name": host,
        "status_code": STATUS_CODES.choose(rng).unwrap().to_string(),
        "ta": rng.gen_range(0..200).to_string(),
        "tc": rng.gen_range(0..200).to_string(),
        "termination_state": "--",
        "tr_client": rng.gen_range(0..200).to_string(),
        "tr_server": rng.gen_range(0..200).to_string(),
        "tw": rng.gen_range(0..200).to_string(),
    });

    serde_json::to_string(&record).expect("serializing log line")
}

fn random_ip(rng: &mut impl Rng) -> String {
    if rng.gen_bool(0.5) {
        format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..=255),
            rng.gen_range(0..=255),
            rng.gen_range(0..=255),
            rng.gen_range(1..=255)
        )
    } else {
        let segments: Vec<String> = (0..8)
            .map(|_| format!("{:x}", rng.gen_range(0u16..=u16::MAX)))
            .collect();
        segments.join(":")
    }
}

fn random_host(rng: &mut impl Rng) -> String {
    let prefix = HOST_PREFIXES.choose(rng).unwrap();
    let suffix: u16 = rng.gen_range(1..=9999);
    format!("{prefix}-{suffix}")
}

fn random_path(rng: &mut impl Rng) -> String {
    let segments = rng.gen_range(2..=4);
    let mut path_parts = Vec::with_capacity(segments);
    for _ in 0..segments {
        path_parts.push((*PATH_SEGMENTS.choose(rng).unwrap()).to_string());
    }
    format!("/{}", path_parts.join("/"))
}

fn random_trace_id(rng: &mut impl Rng) -> String {
    Alphanumeric.sample_string(rng, 8)
}
