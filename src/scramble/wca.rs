use super::WcaEvent;
use jni::objects::{JString, JValue};
use jni::vm::{InitArgsBuilder, JavaVM};
use jni::{JNIVersion, errors, jni_sig, jni_str};
use std::sync::OnceLock;

static JVM: OnceLock<Result<JavaVM, String>> = OnceLock::new();

pub fn get_wca_scramble(event: WcaEvent) -> Option<String> {
    let event_str = event_to_string(event);
    let jvm = get_or_init_jvm().as_ref().ok()?;

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
        .ok()?;

    Some(result)
}

fn get_or_init_jvm() -> &'static Result<JavaVM, String> {
    JVM.get_or_init(|| {
        let classpath = concat!(env!("OUT_DIR"), "/lib-all.jar");
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V21)
            .option(format!("-Djava.class.path={classpath}"))
            .build()
            .map_err(|e| format!("failed to build JVM init args: {e}"))?;

        JavaVM::new(jvm_args).map_err(|e| format!("failed to create JVM: {e}"))
    })
}

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
        WcaEvent::Skewb => "skewb",
        WcaEvent::Square1 => "sq1",
        WcaEvent::Clock => "clock",
    }
}
