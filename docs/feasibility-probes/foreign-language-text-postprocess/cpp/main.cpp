#include "text_postprocess_module_cpp.h"

namespace exports::slicer::postpass_text_postprocess::text_postprocess {
std::expected<wit::string, ::slicer::common::module_errors::ModuleError> Run(
    wit::string, ::slicer::config::config_types::ConfigView&&) {
  constexpr char output[] = ";; foreign-language-probe\n; probe input\n";
  return wit::string::from_view(std::string_view(output, sizeof(output) - 1));
}
}
