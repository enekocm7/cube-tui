use super::WcaEvent;
use jni::objects::{JString, JValue};
use jni::vm::{InitArgsBuilder, JavaVM};
use jni::{JNIVersion, errors, jni_sig, jni_str};
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::id;
use std::sync::OnceLock;

const SCRAMBLE_JAR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lib-all.jar"));

static JVM: OnceLock<Result<JavaVM, String>> = OnceLock::new();

/// Generates an official scramble through the bundled TNoodle-compatible JAR.
pub fn get_wca_scramble(event: WcaEvent) -> Result<String, String> {
    let event_str = event_to_string(event);
    let jvm = get_or_init_jvm().as_ref().map_err(Clone::clone)?;

    let result = jvm
        .attach_current_thread(|env| -> errors::Result<String> {
            let input = JString::new(env, event_str)?;
            let arg = JValue::Object(input.as_ref());
            let value = env.call_static_method(
                jni_str!("org/example/Library"),
                jni_str!("generateScramble"),
                jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                &[arg],
            )?;

            let obj = value.l()?;
            let output = env.cast_local::<JString>(obj)?;
            output.try_to_string(env)
        })
        .map_err(|error| error.to_string())?;

    Ok(result)
}

/// Returns the lazily initialized JVM, retaining any initialization error.
fn get_or_init_jvm() -> &'static Result<JavaVM, String> {
    JVM.get_or_init(|| -> Result<JavaVM, String> {
        let jar_path = extract_jar_to_temp().map_err(|e| format!("failed to extract jar: {e}"))?;
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V21)
            .option(format!("-Djava.class.path={}", jar_path.display()))
            // The embedded scrambler does not need the JVM's server-sized
            // default heap and parallel garbage collector. The bounded heap
            // leaves room for the native app and other WCA puzzles.
            .option("-Xms8m")
            .option("-Xmx64m")
            .option("-XX:+UseSerialGC")
            .option("-XX:ActiveProcessorCount=2")
            .build()
            .map_err(|e| format!("failed to build JVM init args: {e}"))?;

        JavaVM::new(jvm_args).map_err(|e| format!("failed to create JVM: {e}"))
    })
}

/// Writes the embedded scrambler JAR to a process-scoped temporary file.
fn extract_jar_to_temp() -> std::io::Result<PathBuf> {
    let mut path = temp_dir();
    path.push(format!("cube-tui-scrambles-{}.jar", id()));
    let mut file = File::create(&path)?;
    file.write_all(SCRAMBLE_JAR)?;
    Ok(path)
}

/// Maps an event to the identifier expected by the Java scrambler.
const fn event_to_string(event: WcaEvent) -> &'static str {
    match event {
        WcaEvent::Cube2x2 => "222",
        WcaEvent::Cube3x3 => "333",
        WcaEvent::Cube4x4 => "444",
        WcaEvent::Cube5x5 => "555",
        WcaEvent::Cube6x6 => "666",
        WcaEvent::Cube7x7 => "777",
        WcaEvent::Megaminx => "minx",
        WcaEvent::Pyraminx => "pyram",
        WcaEvent::Fto => "fto",
        WcaEvent::Skewb => "skewb",
        WcaEvent::Square1 => "sq1",
        WcaEvent::Clock => "clock",
    }
}

#[cfg(test)]
mod tests {
    use super::{WcaEvent, get_wca_scramble};

    #[test]
    fn bounded_jvm_generates_every_supported_event() {
        for event in [
            WcaEvent::Cube2x2,
            WcaEvent::Cube3x3,
            WcaEvent::Cube4x4,
            WcaEvent::Cube5x5,
            WcaEvent::Cube6x6,
            WcaEvent::Cube7x7,
            WcaEvent::Megaminx,
            WcaEvent::Pyraminx,
            WcaEvent::Skewb,
            WcaEvent::Square1,
            WcaEvent::Clock,
        ] {
            get_wca_scramble(event)
                .unwrap_or_else(|error| panic!("WCA scrambler failed for {event:?}: {error}"));
        }
    }
}
