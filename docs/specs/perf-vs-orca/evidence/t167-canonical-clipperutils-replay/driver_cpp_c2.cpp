// t167: C++ Clipper2 1.5.2 twin driver (continuity with t69, extended with opcode 5 opening_ex and area reporting).
#include "clipper2/clipper.h"
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

using namespace Clipper2Lib;
using U64 = uint64_t;

static void require(bool value, const char* message) {
    if (!value) throw std::runtime_error(message);
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
    Paths64 paths() {
        Paths64 out;
        auto count = number();
        for (U64 p = 0; p < count; ++p) {
            auto nr = number();
            for (U64 r = 0; r < nr; ++r) {
                Path64 ring;
                auto np = number();
                ring.reserve(static_cast<size_t>(np));
                for (U64 i = 0; i < np; ++i) {
                    auto x = signed_number();
                    auto y = signed_number();
                    ring.emplace_back(x, y);
                }
                out.push_back(std::move(ring));
            }
        }
        return out;
    }
};

static double from_bits(U64 bits) {
    double d = 0.0;
    std::memcpy(&d, &bits, 8);
    return d;
}

static double signed_area(const Path64& ring) {
    if (ring.size() < 3) return 0.0;
    long double a = 0.0;
    const size_t n = ring.size();
    for (size_t i = 0; i < n; ++i) {
        size_t j = (i + 1 == n) ? 0 : i + 1;
        a += static_cast<long double>(ring[i].x) * static_cast<long double>(ring[j].y)
           - static_cast<long double>(ring[j].x) * static_cast<long double>(ring[i].y);
    }
    return static_cast<double>(a * 0.5L);
}

static void collect(const PolyPath64& node, std::vector<Paths64>& polys) {
    Paths64 poly;
    poly.push_back(node.Polygon());
    for (size_t h = 0; h < node.Count(); ++h) {
        const auto& hole = *node.Child(h);
        poly.push_back(hole.Polygon());
        for (size_t i = 0; i < hole.Count(); ++i) collect(*hole.Child(i), polys);
    }
    if (poly[0].size() >= 3) polys.push_back(std::move(poly));
}

static std::vector<Paths64> tree_to_polys(const PolyTree64& tree) {
    std::vector<Paths64> polys;
    for (size_t i = 0; i < tree.Count(); ++i) collect(*tree.Child(i), polys);
    return polys;
}

struct Case {
    std::string site;
    int64_t layer = 0;
    U64 opcode = 0;
    std::map<U64, U64> args;
    Paths64 subj, clip;

    U64 arg(U64 id) const { return args.at(id); }
    U64 optional(U64 id, U64 fallback) const {
        auto i = args.find(id);
        return i == args.end() ? fallback : i->second;
    }
    bool flag(U64 id, U64 fallback) const {
        auto v = optional(id, fallback);
        require(v <= 1, "invalid flag");
        return v != 0;
    }
    void validate() const {
        require(opcode <= 5, "unknown opcode");
        for (auto a : args) {
            require((opcode == 0 || opcode == 5)
                        ? ((a.first >= 1 && a.first <= 5) || a.first == 7 || a.first == 8)
                        : (a.first == 10 || a.first == 7 || a.first == 8),
                    "unexpected argument ID");
        }
        flag(7, (opcode >= 1 && opcode <= 4) ? 1 : 0);
        flag(8, 0);
        if (opcode == 0 || opcode == 5) {
            require(arg(2) <= 3 && arg(3) <= 4, "invalid offset enum");
            require(std::isfinite(from_bits(arg(1))) && std::isfinite(from_bits(arg(4))) &&
                        std::isfinite(from_bits(optional(5, 0))),
                    "invalid f64");
        } else {
            require(arg(10) <= 3, "invalid fill rule");
        }
    }
    std::vector<Paths64> run() const {
        PolyTree64 tree;
        if (opcode == 0) {
            double delta = from_bits(arg(1));
            if (delta == 0.0 || subj.empty()) return {};
            ClipperOffset engine(from_bits(arg(4)), from_bits(optional(5, 0)), flag(7, 0), flag(8, 0));
            engine.AddPaths(subj, static_cast<JoinType>(arg(2)), static_cast<EndType>(arg(3)));
            engine.Execute(delta, tree);
            require(engine.ErrorCode() == 0, "offset engine failure");
            return tree_to_polys(tree);
        } else if (opcode == 5) {
            double r = from_bits(arg(1));
            if (r <= 0.0 || subj.empty()) return {};
            double miter = from_bits(arg(4));
            double arc = from_bits(optional(5, 0));
            auto join = static_cast<JoinType>(arg(2));
            auto end = static_cast<EndType>(arg(3));
            Paths64 first;
            {
                ClipperOffset e1(miter, arc, false, false);
                e1.AddPaths(subj, join, end);
                e1.Execute(-r, first);
                require(e1.ErrorCode() == 0, "opening pass1 failure");
            }
            if (first.empty()) return {};
            ClipperOffset e2(miter, arc, false, false);
            e2.AddPaths(first, join, end);
            e2.Execute(r, tree);
            require(e2.ErrorCode() == 0, "opening pass2 failure");
            return tree_to_polys(tree);
        } else {
            Clipper64 engine;
            engine.PreserveCollinear(flag(7, 1));
            engine.ReverseSolution(flag(8, 0));
            engine.AddSubject(subj);
            engine.AddClip(clip);
            const ClipType ops[] = {ClipType::NoClip, ClipType::Difference, ClipType::Union,
                                    ClipType::Intersection, ClipType::Xor};
            require(engine.Execute(ops[opcode], static_cast<FillRule>(arg(10)), tree),
                    "boolean engine failure");
            return tree_to_polys(tree);
        }
    }
};

static auto summary(const std::vector<Paths64>& polys) {
    U64 hash = 14695981039346656037ULL, rings = 0, verts = 0;
    double area = 0.0;
    for (const auto& poly : polys) {
        for (size_t ri = 0; ri < poly.size(); ++ri) {
            const auto& ring = poly[ri];
            ++rings;
            double a = std::abs(signed_area(ring));
            area += (ri == 0) ? a : -a;
            for (const auto& pt : ring) {
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
                for (;;) {
                    auto record = t.next();
                    if (record == "arg") {
                        auto id = t.number();
                        auto value = t.number();
                        require(c.args.emplace(id, value).second, "duplicate arg");
                    } else if (record == "subj") {
                        require(!subj, "duplicate subj");
                        subj = true;
                        c.subj = t.paths();
                    } else if (record == "clip") {
                        require(!clip, "duplicate clip");
                        clip = true;
                        c.clip = t.paths();
                    } else if (record == "end") break;
                    else throw std::runtime_error("unknown record");
                }
                bool expect_clip = (c.opcode >= 1 && c.opcode <= 4);
                require(subj && clip == expect_clip, "incorrect geometry records");
                c.validate();
                size_t in_verts = 0;
                for (const auto& r : c.subj) in_verts += r.size();
                for (const auto& r : c.clip) in_verts += r.size();
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
