// Reference implementation for the generated `run` stub in
// bindings/exports/slicer$postpass_text_postprocess$text_postprocess$1_0_0.ts.
// build.sh splices this body into the generated stub after generation
// (generation rewrites the stub every run, so the splice is repeatable).
export function run(
  gcodeText: string,
  config: ConfigView,
): Result<string, ModuleError> {
  return new Result<string, ModuleError>(
    0, // ok
    ";; foreign-language-probe\n" + gcodeText,
    changetype<ModuleError>(0),
  );
}
