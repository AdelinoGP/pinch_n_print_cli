// t167: Canonical OrcaSlicer v2.5.0 (40ecc66f78) ClipperUtils replay driver.
// Compiles directly against D:\slicerProject\Orca(pnp_gui)\deps_src\clipper2 (Clipper2 2.0.1)
// and reproduces src/libslic3r/ClipperUtils.cpp's exact wrapper logic verbatim:
// - 1 nm coordinates (PnP 100 nm coordinates scaled x100 outside the timer)
// - append_offset_path decimation (ClipperOffsetShortestEdgeFactor = 0.005)
// - c2_offset_setup, c2_offset_as_group, c2_offset_closed_paths, c2_offset_expolygons
// - offset2_ex (c2_offset_expolygons / c2_offset_closed -> Paths64 -> c2_offset_closed -> PolyTree64)
// - c2_clip (PreserveCollinear(false))
// - c2_to_expolygons with orientation-based stray-hole attachment
#include "clipper2/clipper.h"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <map>
#include <sstream>
#include <stdexcept>
#include <tuple>
#include <vector>

namespace C2 = Clipper2Lib;
using U64 = uint64_t;

static constexpr int64_t ORCA_SCALE = 100; // 100 nm -> 1 nm
static constexpr double CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR = 0.005;

static void require(bool value, const char* message) {
    if (!value) throw std::runtime_error(message);
}

struct ExPoly {
    C2::Path64 contour;
    C2::Paths64 holes;
};

static double from_bits(U64 bits) {
    double d = 0.0;
    std::memcpy(&d, &bits, 8);
    return d;
}

static double signed_area_u2(const C2::Path64& ring) {
    if (ring.size() < 3) return 0.0;
    long double a = 0.0;
    const size_t n = ring.size();
    for (size_t i = 0; i < n; ++i) {
        size_t j = (i + 1 == n) ? 0 : i + 1;
        a += static_cast<long double>(ring[i].x) * static_cast<long double>(ring[j].y)
           - static_cast<long double>(ring[j].x) * static_cast<long double>(ring[i].y);
    }
    return static_cast<double>((a * 0.5L) / static_cast<long double>(ORCA_SCALE * ORCA_SCALE));
}

// --- Exact v2.5.0 ClipperUtils.cpp helpers ---

static bool append_offset_path(C2::Paths64& out, const C2::Path64& path, double shortest, C2::EndType end) {
    int high = static_cast<int>(path.size()) - 1;
    if (high < 0) return false;
    const double shortest2 = shortest * shortest;
    auto same = [shortest2](const C2::Point64& a, const C2::Point64& b) {
        const double dx = static_cast<double>(a.x - b.x);
        const double dy = static_cast<double>(a.y - b.y);
        return shortest2 > 0.0 ? (dx * dx + dy * dy < shortest2) : (dx == 0.0 && dy == 0.0);
    };
    if (end == C2::EndType::Polygon || end == C2::EndType::Joined) {
        while (high > 0 && same(path[high], path[0])) --high;
    }
    C2::Path64 dst;
    dst.reserve(static_cast<size_t>(high + 1));
    dst.emplace_back(path[0].x, path[0].y);
    for (int i = 1, last = 0; i <= high; ++i) {
        if (!same(path[i], path[last])) {
            dst.emplace_back(path[i].x, path[i].y);
            last = i;
        }
    }
    if (end == C2::EndType::Polygon && dst.size() < 3) return false;
    out.emplace_back(std::move(dst));
    return true;
}

static void c2_offset_setup(C2::ClipperOffset& co, double delta, C2::JoinType join, double miterLimit, double arcTol) {
    const double max_arc_tolerance = std::abs(delta) * 0.25;
    if (join == C2::JoinType::Round) {
        // Match canonical ClipperUtils c2_offset_setup; if corpus leaves arcTol == 0 (Clipper2 default
        // 0.002*|delta|), use std::min(0.002*|delta|, max_arc_tolerance) so round curve discretization matches.
        double tol = (arcTol > 0.0) ? arcTol : (std::abs(delta) * 0.002);
        co.ArcTolerance(std::min(tol, max_arc_tolerance));
    } else {
        co.MiterLimit(std::max(miterLimit, 2.0));
        co.ArcTolerance(std::min(arcTol > 0.0 ? arcTol : 0.25, max_arc_tolerance));
    }
}

static bool c2_offset_as_group(const C2::Paths64& paths, const std::vector<double>& areas) {
    std::vector<C2::Rect64> contours;
    for (size_t i = 0; i < paths.size(); ++i)
        if (areas[i] > 0.0)
            contours.emplace_back(C2::GetBounds(paths[i]));
    for (size_t i = 0; i < paths.size(); ++i)
        if (areas[i] < 0.0) {
            const C2::Rect64 hole = C2::GetBounds(paths[i]);
            if (std::none_of(contours.begin(), contours.end(), [&hole](const C2::Rect64& r) {
                    return r.left < hole.left && r.right > hole.right &&
                           r.top < hole.top && r.bottom > hole.bottom;
                }))
                return false;
        }
    return true;
}

template <class TOut>
static void c2_clip(C2::ClipType type, const C2::Paths64& subject, const C2::Paths64& clip,
                    C2::FillRule fill, TOut& out) {
    C2::Clipper64 clipper;
    clipper.PreserveCollinear(false);
    clipper.AddSubject(subject);
    if (!clip.empty()) clipper.AddClip(clip);
    require(clipper.Execute(type, fill, out), "c2_clip failed");
}

template <class TOut>
static void c2_offset_closed_paths(C2::Paths64&& paths, double delta, C2::JoinType join,
                                   double miterLimit, double arcTol, TOut& out) {
    std::vector<double> areas;
    areas.reserve(paths.size());
    for (const C2::Path64& path : paths) areas.emplace_back(C2::Area(path));
    C2::ClipperOffset co;
    c2_offset_setup(co, delta, join, miterLimit, arcTol);
    if (c2_offset_as_group(paths, areas)) {
        if (delta < 0.0 && std::all_of(areas.begin(), areas.end(), [](double a) { return a == 0.0; }))
            return;
        co.AddPaths(paths, join, C2::EndType::Polygon);
        co.Execute(delta, out);
        return;
    }
    C2::Paths64 offsetted, single;
    for (size_t i = 0; i < paths.size(); ++i) {
        if (areas[i] == 0.0 && delta < 0.0) continue;
        co.Clear();
        co.AddPath(paths[i], join, C2::EndType::Polygon);
        co.Execute(areas[i] < 0.0 ? -delta : delta, single);
        for (auto& p : single) offsetted.emplace_back(std::move(p));
        single.clear();
    }
    c2_clip(C2::ClipType::Union, offsetted, {}, delta > 0.0 ? C2::FillRule::NonZero : C2::FillRule::Positive, out);
}

template <class TOut>
static void c2_offset_closed(const C2::Paths64& src, double delta, C2::JoinType join,
                             double miterLimit, double arcTol, TOut& out) {
    const double shortest = std::abs(delta * CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR);
    C2::Paths64 paths;
    paths.reserve(src.size());
    for (const auto& path : src)
        append_offset_path(paths, path, shortest, C2::EndType::Polygon);
    c2_offset_closed_paths(std::move(paths), delta, join, miterLimit, arcTol, out);
}

template <class TOut>
static void c2_offset_expolygons(const std::vector<ExPoly>& src, double delta, C2::JoinType join,
                                 double miterLimit, double arcTol, TOut& out) {
    const double shortest = std::abs(delta * CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR);
    C2::Paths64 paths;
    for (const auto& expoly : src) {
        if (!append_offset_path(paths, expoly.contour, shortest, C2::EndType::Polygon)) continue;
        const double area = C2::Area(paths.back());
        if (area == 0.0) {
            if (delta < 0.0) paths.pop_back();
            continue;
        } else if (area < 0.0) {
            std::reverse(paths.back().begin(), paths.back().end());
        }
        for (const auto& hole : expoly.holes) {
            if (append_offset_path(paths, hole, shortest, C2::EndType::Polygon)) {
                const double ha = C2::Area(paths.back());
                if (ha == 0.0) paths.pop_back();
                else if (ha > 0.0) std::reverse(paths.back().begin(), paths.back().end());
            }
        }
    }
    C2::ClipperOffset co;
    c2_offset_setup(co, delta, join, miterLimit, arcTol);
    co.AddPaths(paths, join, C2::EndType::Polygon);
    co.Execute(delta, out);
}

static void c2_append_expolygons(const C2::PolyPath64& outer, std::vector<ExPoly>& out,
                                 std::vector<const C2::Path64*>& stray_holes) {
    if (C2::Area(outer.Polygon()) < 0.0) {
        stray_holes.emplace_back(&outer.Polygon());
        for (const auto& island : outer) c2_append_expolygons(*island, out, stray_holes);
        return;
    }
    const size_t idx = out.size();
    out.emplace_back().contour = outer.Polygon();
    out[idx].holes.reserve(outer.Count());
    for (const auto& child : outer) {
        if (C2::Area(child->Polygon()) > 0.0) {
            c2_append_expolygons(*child, out, stray_holes);
        } else {
            out[idx].holes.emplace_back(child->Polygon());
            for (const auto& island : *child) c2_append_expolygons(*island, out, stray_holes);
        }
    }
}

static void c2_attach_stray_holes(std::vector<ExPoly>& out, const std::vector<const C2::Path64*>& stray_holes) {
    if (stray_holes.empty()) return;
    std::vector<C2::Rect64> bboxes;
    std::vector<double> areas;
    bboxes.reserve(out.size());
    areas.reserve(out.size());
    for (const auto& e : out) {
        bboxes.push_back(C2::GetBounds(e.contour));
        areas.push_back(C2::Area(e.contour));
    }
    for (const C2::Path64* h : stray_holes) {
        const double ha = -C2::Area(*h);
        const C2::Rect64 hb = C2::GetBounds(*h);
        size_t best = static_cast<size_t>(-1);
        double best_area = 1e300;
        for (size_t i = 0; i < out.size(); ++i) {
            if (areas[i] <= ha || areas[i] >= best_area) continue;
            if (!(bboxes[i].left <= hb.left && bboxes[i].right >= hb.right &&
                  bboxes[i].top <= hb.top && bboxes[i].bottom >= hb.bottom))
                continue;
            bool enclosed = false;
            for (const auto& pt : *h) {
                auto r = C2::PointInPolygon(pt, out[i].contour);
                if (r != C2::PointInPolygonResult::IsOn) {
                    enclosed = (r == C2::PointInPolygonResult::IsInside);
                    break;
                }
            }
            if (enclosed) {
                best = i;
                best_area = areas[i];
            }
        }
        if (best != static_cast<size_t>(-1)) out[best].holes.push_back(*h);
        else out.emplace_back().contour = *h;
    }
}

static std::vector<ExPoly> c2_to_expolygons(const C2::PolyTree64& tree, size_t& stray_count) {
    std::vector<ExPoly> out;
    std::vector<const C2::Path64*> stray;
    for (const auto& outer : tree) c2_append_expolygons(*outer, out, stray);
    stray_count = stray.size();
    c2_attach_stray_holes(out, stray);
    return out;
}

struct Tokens {
    std::vector<std::string> values;
    size_t pos = 0;
    std::string next() { require(pos < values.size(), "truncated corpus"); return values[pos++]; }
    U64 number() {
        auto s = next();
        require(!s.empty() && s[0] != '-', "invalid unsigned integer");
        size_t n = 0;
        auto v = std::stoull(s, &n);
        require(n == s.size(), "invalid integer");
        return v;
    }
    int64_t signed_number() {
        auto s = next();
        size_t n = 0;
        auto v = std::stoll(s, &n);
        require(n == s.size(), "invalid integer");
        return v;
    }
    std::vector<ExPoly> read_expolys(C2::Paths64& flat_out, bool& is_expoly_framed) {
        std::vector<ExPoly> exps;
        auto count = number();
        bool any_multi_ring = false;
        bool any_neg_ring0 = false;
        for (U64 p = 0; p < count; ++p) {
            auto nr = number();
            if (nr > 1) any_multi_ring = true;
            ExPoly ep;
            for (U64 r = 0; r < nr; ++r) {
                C2::Path64 ring;
                auto np = number();
                ring.reserve(static_cast<size_t>(np));
                for (U64 i = 0; i < np; ++i) {
                    int64_t x = signed_number() * ORCA_SCALE;
                    int64_t y = signed_number() * ORCA_SCALE;
                    ring.emplace_back(x, y);
                }
                if (r == 0) {
                    if (C2::Area(ring) < 0.0) any_neg_ring0 = true;
                    ep.contour = ring;
                } else {
                    ep.holes.push_back(ring);
                }
                flat_out.push_back(std::move(ring));
            }
            if (nr > 0) exps.push_back(std::move(ep));
        }
        is_expoly_framed = any_multi_ring || !any_neg_ring0;
        return exps;
    }
};

struct Case {
    std::string site;
    int64_t layer = 0;
    U64 opcode = 0;
    std::map<U64, U64> args;
    std::vector<ExPoly> subj_ex;
    C2::Paths64 subj_flat, clip_flat;
    bool subj_is_ex = false;

    U64 arg(U64 id) const { return args.at(id); }
    U64 optional(U64 id, U64 fallback) const {
        auto i = args.find(id);
        return i == args.end() ? fallback : i->second;
    }
    void validate() const {
        require(opcode <= 5, "unknown opcode");
        if (opcode == 0 || opcode == 5) {
            require(arg(2) <= 3 && arg(3) <= 4, "invalid offset enum");
        } else {
            require(arg(10) <= 3, "invalid fill rule");
        }
    }
    C2::JoinType join() const {
        return static_cast<C2::JoinType>(arg(2));
    }
    std::vector<ExPoly> run(size_t& stray_count) const {
        C2::PolyTree64 tree;
        stray_count = 0;
        if (opcode == 0) {
            double delta = from_bits(arg(1)) * ORCA_SCALE;
            if (delta == 0.0 || subj_flat.empty()) return {};
            double miter = from_bits(arg(4));
            double arc = from_bits(optional(5, 0)) * ORCA_SCALE;
            if (subj_is_ex) c2_offset_expolygons(subj_ex, delta, join(), miter, arc, tree);
            else c2_offset_closed(subj_flat, delta, join(), miter, arc, tree);
            return c2_to_expolygons(tree, stray_count);
        } else if (opcode == 5) {
            double r = from_bits(arg(1)) * ORCA_SCALE;
            if (r <= 0.0 || subj_flat.empty()) return {};
            double miter = from_bits(arg(4));
            double arc = from_bits(optional(5, 0)) * ORCA_SCALE;
            C2::Paths64 first;
            if (subj_is_ex) c2_offset_expolygons(subj_ex, -r, join(), miter, arc, first);
            else c2_offset_closed(subj_flat, -r, join(), miter, arc, first);
            if (first.empty()) return {};
            c2_offset_closed(first, r, join(), miter, arc, tree);
            return c2_to_expolygons(tree, stray_count);
        } else {
            const C2::ClipType ops[] = {C2::ClipType::NoClip, C2::ClipType::Difference,
                                        C2::ClipType::Union, C2::ClipType::Intersection,
                                        C2::ClipType::Xor};
            c2_clip(ops[opcode], subj_flat, clip_flat, static_cast<C2::FillRule>(arg(10)), tree);
            return c2_to_expolygons(tree, stray_count);
        }
    }
};

static auto summary(const std::vector<ExPoly>& polys) {
    U64 hash = 14695981039346656037ULL, rings = 0, verts = 0;
    double area = 0.0;
    for (const auto& ep : polys) {
        ++rings;
        area += std::abs(signed_area_u2(ep.contour));
        for (const auto& pt : ep.contour) {
            ++verts;
            for (U64 value : {static_cast<U64>(pt.x), static_cast<U64>(pt.y)}) {
                for (int b = 0; b < 8; ++b) {
                    hash ^= (value >> (8 * b)) & 255;
                    hash *= 1099511628211ULL;
                }
            }
        }
        for (const auto& h : ep.holes) {
            ++rings;
            area -= std::abs(signed_area_u2(h));
            for (const auto& pt : h) {
                ++verts;
                for (U64 value : {static_cast<U64>(pt.x), static_cast<U64>(pt.y)}) {
                    for (int b = 0; b < 8; ++b) {
                        hash ^= (value >> (8 * b)) & 255;
                        hash *= 1099511628211ULL;
                    }
                }
            }
        }
    }
    return std::make_tuple(polys.size(), rings, verts, area, hash);
}

static std::string json_string(const std::string& s) {
    std::ostringstream o;
    o << '"';
    for (unsigned char c : s) {
        if (c == '"' || c == '\\') o << '\\' << c;
        else if (c < 32) {
            const char* hex = "0123456789abcdef";
            o << "\\u00" << hex[c >> 4] << hex[c & 15];
        } else o << c;
    }
    o << '"';
    return o.str();
}

int main(int argc, char** argv) {
    try {
        require(argc > 1, "usage: driver corpus.cases ...");
        for (int f = 1; f < argc; ++f) {
            std::ifstream stream(argv[f]);
            require(stream.good(), "cannot read corpus");
            Tokens t;
            std::string line;
            while (std::getline(stream, line)) {
                line = line.substr(0, line.find('#'));
                std::istringstream in(line);
                std::string token;
                while (in >> token) t.values.push_back(token);
            }
            size_t index = 0;
            while (t.pos < t.values.size()) {
                require(t.next() == "case", "expected case");
                Case c;
                c.site = t.next();
                c.layer = t.signed_number();
                c.opcode = t.number();
                if (t.pos < t.values.size() && !t.values[t.pos].empty() &&
                    std::isdigit(static_cast<unsigned char>(t.values[t.pos][0]))) {
                    t.number();
                }
                bool subj = false, clip = false;
                bool dummy = false;
                for (;;) {
                    auto record = t.next();
                    if (record == "arg") {
                        auto id = t.number();
                        auto value = t.number();
                        require(c.args.emplace(id, value).second, "duplicate arg");
                    } else if (record == "subj") {
                        require(!subj, "duplicate subj");
                        subj = true;
                        c.subj_ex = t.read_expolys(c.subj_flat, c.subj_is_ex);
                    } else if (record == "clip") {
                        require(!clip, "duplicate clip");
                        clip = true;
                        (void)t.read_expolys(c.clip_flat, dummy);
                    } else if (record == "end") break;
                    else throw std::runtime_error("unknown record");
                }
                bool expect_clip = (c.opcode >= 1 && c.opcode <= 4);
                require(subj && clip == expect_clip, "incorrect geometry records");
                c.validate();
                size_t in_verts = 0;
                for (const auto& r : c.subj_flat) in_verts += r.size();
                for (const auto& r : c.clip_flat) in_verts += r.size();
                size_t stray = 0;
                auto warm = c.run(stray);
                auto expected = summary(warm);
                warm.clear();
                std::vector<U64> times;
                for (int r = 0; r < 5; ++r) {
                    size_t st = 0;
                    auto start = std::chrono::steady_clock::now();
                    auto output = c.run(st);
                    auto finish = std::chrono::steady_clock::now();
                    times.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(finish - start).count());
                    auto got = summary(output);
                    require(std::get<0>(got) == std::get<0>(expected) &&
                                std::get<1>(got) == std::get<1>(expected) &&
                                std::get<2>(got) == std::get<2>(expected) &&
                                std::get<4>(got) == std::get<4>(expected),
                            "nondeterministic output");
                }
                std::ostringstream hash;
                hash << std::hex << std::setw(16) << std::setfill('0') << std::get<4>(expected);
                std::cout << "{\"file\":" << json_string(argv[f]) << ",\"case_index\":" << index++
                          << ",\"site\":" << json_string(c.site) << ",\"layer\":" << c.layer
                          << ",\"opcode\":" << c.opcode << ",\"in_verts\":" << in_verts
                          << ",\"stray_holes\":" << stray
                          << ",\"repeat_times_ns\":[";
                for (size_t r = 0; r < times.size(); ++r) {
                    if (r) std::cout << ',';
                    std::cout << times[r];
                }
                std::cout << "],\"out_paths\":" << std::get<0>(expected)
                          << ",\"out_rings\":" << std::get<1>(expected)
                          << ",\"out_verts\":" << std::get<2>(expected)
                          << ",\"out_area_u2\":" << std::setprecision(17) << std::get<3>(expected)
                          << ",\"out_hash\":\"" << hash.str() << "\"}\n";
            }
        }
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
