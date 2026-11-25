// t167: Pre-2.5.0 OrcaSlicer (OrcaSlicerDocumented @ 3e1daccd7c) Clipper1 (ClipperLib 6.2.6)
// ClipperUtils replay driver.
// Directly links OrcaSlicerDocumented/deps_src/clipper/clipper.cpp and implements
// OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp's exact Clipper1 wrapper logic:
// - 1 nm coordinates (PnP 100 nm scaled x100 outside timer)
// - co.ShortestEdgeLength = |delta * 0.005|
// - offset_expolygon_inner (per-contour + per-hole offsetting for ExPolygons)
// - raw_offset + expand_paths / shrink_paths (for flat Polygons)
// - offset2_ex (expolygons_offset pass 1 -> offset_paths pass 2 into PolyTree)
// - clipper_do / clipper_union into PolyTree -> PolyTreeToExPolygons
#include "clipper.hpp"
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

using U64 = uint64_t;
static constexpr int64_t ORCA_SCALE = 100;
static constexpr double CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR = 0.005;

static void require(bool value, const char* message) {
    if (!value) throw std::runtime_error(message);
}

struct ExPoly {
    ClipperLib::Path contour;
    ClipperLib::Paths holes;
};

static double from_bits(U64 bits) {
    double d = 0.0;
    std::memcpy(&d, &bits, 8);
    return d;
}

static double signed_area_u2(const ClipperLib::Path& ring) {
    if (ring.size() < 3) return 0.0;
    long double a = 0.0;
    const size_t n = ring.size();
    for (size_t i = 0; i < n; ++i) {
        size_t j = (i + 1 == n) ? 0 : i + 1;
        a += static_cast<long double>(ring[i].x()) * static_cast<long double>(ring[j].y())
           - static_cast<long double>(ring[j].x()) * static_cast<long double>(ring[i].y());
    }
    return static_cast<double>((a * 0.5L) / static_cast<long double>(ORCA_SCALE * ORCA_SCALE));
}

static void append_paths(ClipperLib::Paths& dst, ClipperLib::Paths&& src) {
    if (dst.empty()) dst = std::move(src);
    else {
        dst.reserve(dst.size() + src.size());
        for (auto& p : src) dst.emplace_back(std::move(p));
    }
}

static ClipperLib::JoinType map_join(U64 j) {
    switch (j) {
        case 0: return ClipperLib::jtSquare;
        case 2: return ClipperLib::jtRound;
        default: return ClipperLib::jtMiter;
    }
}

static ClipperLib::PolyFillType map_fill(U64 f) {
    switch (f) {
        case 0: return ClipperLib::pftEvenOdd;
        case 1: return ClipperLib::pftNonZero;
        case 2: return ClipperLib::pftPositive;
        default: return ClipperLib::pftNegative;
    }
}

static void setup_co(ClipperLib::ClipperOffset& co, double delta, ClipperLib::JoinType jt,
                     double miter, double arc_tol) {
    if (jt == ClipperLib::jtRound) {
        double tol = (arc_tol > 0.0) ? arc_tol : (std::abs(delta) * 0.002);
        co.ArcTolerance = std::min(tol, std::abs(delta) * 0.25);
    } else {
        co.MiterLimit = std::max(miter, 2.0);
    }
    co.ShortestEdgeLength = std::abs(delta * CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR);
}

template <class TOut>
static TOut clipper_do(ClipperLib::ClipType ct, const ClipperLib::Paths& subj,
                       const ClipperLib::Paths& clip, ClipperLib::PolyFillType fill) {
    ClipperLib::Clipper clipper;
    clipper.AddPaths(subj, ClipperLib::ptSubject, true);
    if (!clip.empty()) clipper.AddPaths(clip, ClipperLib::ptClip, true);
    TOut out;
    clipper.Execute(ct, out, fill, fill);
    return out;
}

template <class TOut>
static TOut clipper_union(const ClipperLib::Paths& subj,
                          ClipperLib::PolyFillType fill = ClipperLib::pftNonZero) {
    ClipperLib::Clipper clipper;
    clipper.AddPaths(subj, ClipperLib::ptSubject, true);
    TOut out;
    clipper.Execute(ClipperLib::ctUnion, out, fill, fill);
    return out;
}

static ClipperLib::Paths raw_offset(const ClipperLib::Paths& paths, double offset,
                                    ClipperLib::JoinType jt, double miter, double arc_tol) {
    ClipperLib::ClipperOffset co;
    setup_co(co, offset, jt, miter, arc_tol);
    ClipperLib::Paths out, out_this;
    out.reserve(paths.size());
    for (const auto& path : paths) {
        co.Clear();
        co.AddPath(path, jt, ClipperLib::etClosedPolygon);
        bool ccw = ClipperLib::Orientation(path);
        co.Execute(out_this, ccw ? offset : -offset);
        if (!ccw) {
            for (auto& p : out_this) std::reverse(p.begin(), p.end());
        }
        append_paths(out, std::move(out_this));
        out_this.clear();
    }
    return out;
}

static void remove_outermost(ClipperLib::Paths& solution) {
    if (!solution.empty()) solution.erase(solution.begin());
}
static void remove_outermost(ClipperLib::PolyTree& solution) {
    solution.RemoveOutermostPolygon();
}

template <class TOut>
static TOut offset_paths(const ClipperLib::Paths& paths, double offset,
                         ClipperLib::JoinType jt, double miter, double arc_tol) {
    if (offset > 0.0) {
        return clipper_union<TOut>(raw_offset(paths, offset, jt, miter, arc_tol), ClipperLib::pftNonZero);
    } else if (offset < 0.0) {
        ClipperLib::Paths raw = raw_offset(paths, offset, jt, miter, arc_tol);
        TOut out;
        if (!raw.empty()) {
            ClipperLib::Clipper clipper;
            clipper.AddPaths(raw, ClipperLib::ptSubject, true);
            ClipperLib::IntRect r = clipper.GetBounds();
            ClipperLib::Path frame;
            frame.emplace_back(r.left - 10, r.bottom + 10);
            frame.emplace_back(r.right + 10, r.bottom + 10);
            frame.emplace_back(r.right + 10, r.top - 10);
            frame.emplace_back(r.left - 10, r.top - 10);
            clipper.AddPath(frame, ClipperLib::ptSubject, true);
            clipper.ReverseSolution(true);
            clipper.Execute(ClipperLib::ctUnion, out, ClipperLib::pftNegative, ClipperLib::pftNegative);
            remove_outermost(out);
        }
        return out;
    }
    return clipper_union<TOut>(paths, ClipperLib::pftNonZero);
}

static int offset_expolygon_inner(const ExPoly& expoly, double delta, ClipperLib::JoinType jt,
                                  double miter, double arc_tol, ClipperLib::Paths& out) {
    ClipperLib::Paths contours;
    {
        ClipperLib::ClipperOffset co;
        setup_co(co, delta, jt, miter, arc_tol);
        co.AddPath(expoly.contour, jt, ClipperLib::etClosedPolygon);
        co.Execute(contours, delta);
    }
    if (contours.empty()) return 0;

    if (expoly.holes.empty()) {
        append_paths(out, std::move(contours));
    } else {
        ClipperLib::Paths holes;
        {
            ClipperLib::ClipperOffset co;
            setup_co(co, delta, jt, miter, arc_tol);
            for (const auto& hole : expoly.holes) {
                co.Clear();
                co.AddPath(hole, jt, ClipperLib::etClosedPolygon);
                ClipperLib::Paths out2;
                co.Execute(out2, -delta);
                append_paths(holes, std::move(out2));
            }
        }
        if (holes.empty()) {
            append_paths(out, std::move(contours));
        } else if (delta < 0.0) {
            auto output = clipper_do<ClipperLib::Paths>(ClipperLib::ctDifference, contours, holes,
                                                        ClipperLib::pftNonZero);
            if (!output.empty()) append_paths(out, std::move(output));
            else return 0;
        } else {
            out.reserve(out.size() + contours.size() + holes.size());
            append_paths(out, std::move(contours));
            for (auto& h : holes) std::reverse(h.begin(), h.end());
            append_paths(out, std::move(holes));
        }
    }
    return 1;
}

static std::pair<ClipperLib::Paths, size_t> expolygons_offset_raw(
    const std::vector<ExPoly>& exps, double delta, ClipperLib::JoinType jt, double miter, double arc_tol) {
    ClipperLib::Paths output;
    output.reserve(exps.size());
    size_t collected = 0;
    for (const auto& ep : exps)
        collected += static_cast<size_t>(offset_expolygon_inner(ep, delta, jt, miter, arc_tol, output));
    return {std::move(output), collected};
}

static ClipperLib::Paths expolygons_offset(const std::vector<ExPoly>& exps, double delta,
                                           ClipperLib::JoinType jt, double miter, double arc_tol) {
    auto [output, collected] = expolygons_offset_raw(exps, delta, jt, miter, arc_tol);
    return (collected > 1 && delta > 0.0) ? clipper_union<ClipperLib::Paths>(output) : std::move(output);
}

static void poly_tree_rec(ClipperLib::PolyNode&& node, std::vector<ExPoly>& out) {
    const size_t idx = out.size();
    out.emplace_back();
    out[idx].contour = std::move(node.Contour);
    out[idx].holes.reserve(node.Childs.size());
    for (ClipperLib::PolyNode* child : node.Childs) {
        out[idx].holes.emplace_back(std::move(child->Contour));
        for (ClipperLib::PolyNode* grand : child->Childs)
            poly_tree_rec(std::move(*grand), out);
    }
}

static std::vector<ExPoly> polytree_to_expolygons(ClipperLib::PolyTree&& tree) {
    std::vector<ExPoly> out;
    for (ClipperLib::PolyNode* child : tree.Childs)
        poly_tree_rec(std::move(*child), out);
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
    std::vector<ExPoly> read_expolys(ClipperLib::Paths& flat_out, bool& is_expoly_framed) {
        std::vector<ExPoly> exps;
        auto count = number();
        bool any_multi_ring = false;
        bool any_neg_ring0 = false;
        for (U64 p = 0; p < count; ++p) {
            auto nr = number();
            if (nr > 1) any_multi_ring = true;
            ExPoly ep;
            for (U64 r = 0; r < nr; ++r) {
                ClipperLib::Path ring;
                auto np = number();
                ring.reserve(static_cast<size_t>(np));
                for (U64 i = 0; i < np; ++i) {
                    int64_t x = signed_number() * ORCA_SCALE;
                    int64_t y = signed_number() * ORCA_SCALE;
                    ring.emplace_back(x, y);
                }
                if (r == 0) {
                    if (!ClipperLib::Orientation(ring)) any_neg_ring0 = true;
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
    ClipperLib::Paths subj_flat, clip_flat;
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
    std::vector<ExPoly> run() const {
        if (opcode == 0) {
            double delta = from_bits(arg(1)) * ORCA_SCALE;
            if (delta == 0.0 || subj_flat.empty()) return {};
            auto jt = map_join(arg(2));
            double miter = from_bits(arg(4));
            double arc = from_bits(optional(5, 0)) * ORCA_SCALE;
            if (subj_is_ex) {
                auto [raw, _] = expolygons_offset_raw(subj_ex, delta, jt, miter, arc);
                return polytree_to_expolygons(clipper_union<ClipperLib::PolyTree>(raw));
            } else {
                return polytree_to_expolygons(offset_paths<ClipperLib::PolyTree>(subj_flat, delta, jt, miter, arc));
            }
        } else if (opcode == 5) {
            double r = from_bits(arg(1)) * ORCA_SCALE;
            if (r <= 0.0 || subj_flat.empty()) return {};
            auto jt = map_join(arg(2));
            double miter = from_bits(arg(4));
            double arc = from_bits(optional(5, 0)) * ORCA_SCALE;
            ClipperLib::Paths first = subj_is_ex
                ? expolygons_offset(subj_ex, -r, jt, miter, arc)
                : offset_paths<ClipperLib::Paths>(subj_flat, -r, jt, miter, arc);
            if (first.empty()) return {};
            return polytree_to_expolygons(offset_paths<ClipperLib::PolyTree>(first, r, jt, miter, arc));
        } else {
            const ClipperLib::ClipType ops[] = {ClipperLib::ctIntersection, ClipperLib::ctDifference,
                                                ClipperLib::ctUnion, ClipperLib::ctIntersection,
                                                ClipperLib::ctXor};
            auto fill = map_fill(arg(10));
            return polytree_to_expolygons(clipper_do<ClipperLib::PolyTree>(ops[opcode], subj_flat, clip_flat, fill));
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
            for (U64 value : {static_cast<U64>(pt.x()), static_cast<U64>(pt.y())}) {
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
                for (U64 value : {static_cast<U64>(pt.x()), static_cast<U64>(pt.y())}) {
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
                auto warm = c.run();
                auto expected = summary(warm);
                warm.clear();
                std::vector<U64> times;
                for (int r = 0; r < 5; ++r) {
                    auto start = std::chrono::steady_clock::now();
                    auto output = c.run();
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
