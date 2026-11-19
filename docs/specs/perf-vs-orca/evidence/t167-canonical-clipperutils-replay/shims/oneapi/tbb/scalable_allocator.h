#pragma once
// Standalone compatibility shim for <oneapi/tbb/scalable_allocator.h>
// Used only by the t167 microbenchmark drivers to compile OrcaSlicer's bundled
// Clipper2 2.0.1 and Clipper1 6.2.6 unmodified with g++ without a full TBB build.
// Allocator difference vs Orca's tbbmalloc is explicitly disclosed in FINDINGS.md.
#include <cstdlib>
#include <memory>

inline void* scalable_malloc(size_t size) { return std::malloc(size); }
inline void  scalable_free(void* ptr) { std::free(ptr); }

namespace tbb {
template <typename T>
using scalable_allocator = std::allocator<T>;
} // namespace tbb
