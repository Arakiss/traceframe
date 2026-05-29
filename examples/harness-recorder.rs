use std::path::PathBuf;

use traceframe::trace::TraceRecorder;

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(".traceframe/runs/example-harness.traceframe");
    let recorder = TraceRecorder::start(&path, "example-harness", true)?;

    recorder.model_call("openai", "gpt-5.5")?;
    recorder.permission_decision("fs.write:README.md", "allow")?;
    recorder.tool_call("shell", "cargo test", ["cargo", "test"])?;
    recorder.tool_result("shell", "cargo test", true, Some(0), Some(42))?;
    recorder.finish("success", Some("example harness run completed"))?;

    print!("{}", recorder.summary()?.render_text());
    Ok(())
}
