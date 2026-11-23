// t70 profiling driver (tier 1, C++ leg): identical semantics to the t69
// driver, plus thread-local counting operator new/delete so allocation volume
// is attributed per case over the SAME measurement window as the Rust tier-1
// driver (engine run + tree summary, excluding tree destruction).
#include "clipper2/clipper.h"
#include <chrono>
#include <cstring>
#include <cstdint>
#include <fstream>
#include <iostream>
#include <map>
#include <new>
#include <sstream>
#include <stdexcept>
#include <tuple>
#include <vector>
using namespace Clipper2Lib;
using U64 = uint64_t;

// ---- counting allocator ---------------------------------------------------
static thread_local U64 g_alloc_count=0, g_alloc_bytes=0;
static void note_alloc(size_t n) { ++g_alloc_count; g_alloc_bytes+=n; }
static void note_dealloc() { }
void* operator new(size_t n) { note_alloc(n); void*p=std::malloc(n?n:1); if(!p) throw std::bad_alloc(); return p; }
void* operator new[](size_t n) { note_alloc(n); void*p=std::malloc(n?n:1); if(!p) throw std::bad_alloc(); return p; }
void operator delete(void*p) noexcept { note_dealloc(); std::free(p); }
void operator delete[](void*p) noexcept { note_dealloc(); std::free(p); }
void operator delete(void*p, size_t) noexcept { note_dealloc(); std::free(p); }
void operator delete[](void*p, size_t) noexcept { note_dealloc(); std::free(p); }
void* operator new(size_t n, std::align_val_t a) { note_alloc(n); void*p=::operator new(n,std::align_val_t(a)); if(!p) throw std::bad_alloc(); return p; }
void* operator new[](size_t n, std::align_val_t a) { note_alloc(n); void*p=::operator new[](n,std::align_val_t(a)); if(!p) throw std::bad_alloc(); return p; }
void operator delete(void*p, std::align_val_t) noexcept { note_dealloc(); std::free(p); }
void operator delete[](void*p, std::align_val_t) noexcept { note_dealloc(); std::free(p); }
void operator delete(void*p, size_t, std::align_val_t) noexcept { note_dealloc(); std::free(p); }
void operator delete[](void*p, size_t, std::align_val_t) noexcept { note_dealloc(); std::free(p); }
static void reset_counters() { g_alloc_count=0; g_alloc_bytes=0; }

static void require(bool value, const char* message) { if (!value) throw std::runtime_error(message); }
struct Tokens {
    std::vector<std::string> values; size_t pos=0;
    std::string next() { require(pos<values.size(),"truncated corpus"); return values[pos++]; }
    U64 number() { auto s=next(); require(!s.empty() && s[0]!='-',"invalid unsigned integer"); size_t n; auto v=std::stoull(s,&n); require(n==s.size(),"invalid integer"); return v; }
    int64_t signed_number() { auto s=next(); size_t n; auto v=std::stoll(s,&n); require(n==s.size(),"invalid integer"); return v; }
    Paths64 paths() {
        Paths64 out; auto count=number();
        for(U64 p=0;p<count;++p) { auto nr=number(); for(U64 r=0;r<nr;++r) {
            Path64 ring; auto np=number(); for(U64 i=0;i<np;++i) { auto x=signed_number(); auto y=signed_number(); ring.emplace_back(x,y); }
            out.push_back(std::move(ring));
        }} return out;
    }
};
static double from_bits(U64 bits) { double d; std::memcpy(&d,&bits,8); return d; }
struct Case {
    std::string site; int64_t layer; U64 opcode; std::map<U64,U64> args; Paths64 subj,clip;
    U64 arg(U64 id) const { return args.at(id); }
    U64 optional(U64 id,U64 fallback) const { auto i=args.find(id); return i==args.end()?fallback:i->second; }
    bool flag(U64 id,U64 fallback) const { auto v=optional(id,fallback); require(v<=1,"invalid flag"); return v!=0; }
    size_t input_verts() const { size_t n=0; for(const auto&p:subj) n+=p.size(); for(const auto&p:clip) n+=p.size(); return n; }
    void validate() const {
        require(opcode<=4,"unknown opcode");
        for(auto a:args) require(opcode==0?((a.first>=1 && a.first<=5)||a.first==7||a.first==8):(a.first==10||a.first==7||a.first==8),"unexpected argument ID");
        flag(7,opcode!=0); flag(8,0);
        if(opcode==0) {
            require(arg(2)<=3 && arg(3)<=4,"invalid offset enum");
            require(std::isfinite(from_bits(arg(1))) && std::isfinite(from_bits(arg(4))) && std::isfinite(from_bits(optional(5,0))),"invalid f64");
        } else require(arg(10)<=3,"invalid fill rule");
    }
    void run(PolyTree64& tree) const {
        if(opcode==0) {
            ClipperOffset engine(from_bits(arg(4)),from_bits(optional(5,0)),flag(7,0),flag(8,0));
            engine.AddPaths(subj,static_cast<JoinType>(arg(2)),static_cast<EndType>(arg(3)));
            engine.Execute(from_bits(arg(1)),tree);
            require(engine.ErrorCode()==0,"offset engine failure");
        } else {
            Clipper64 engine; engine.PreserveCollinear(flag(7,1)); engine.ReverseSolution(flag(8,0));
            engine.AddSubject(subj); engine.AddClip(clip);
            const ClipType ops[]={ClipType::NoClip,ClipType::Difference,ClipType::Union,ClipType::Intersection,ClipType::Xor};
            require(engine.Execute(ops[opcode],static_cast<FillRule>(arg(10)),tree),"boolean engine failure");
        }
    }
};
static void collect(const PolyPath64& node,std::vector<Paths64>& polys) {
    Paths64 poly{node.Polygon()};
    for(size_t h=0;h<node.Count();++h) {
        const auto& hole=*node.Child(h); poly.push_back(hole.Polygon());
        for(size_t i=0;i<hole.Count();++i) collect(*hole.Child(i),polys);
    }
    polys.push_back(std::move(poly));
}
static auto summary(const PolyTree64& tree) {
    std::vector<Paths64> polys; for(size_t i=0;i<tree.Count();++i) collect(*tree.Child(i),polys);
    U64 hash=14695981039346656037ULL, rings=0,verts=0;
    for(const auto& poly:polys) for(const auto& ring:poly) { ++rings; for(const auto& pt:ring) {
        ++verts; for(U64 value:{static_cast<U64>(pt.x),static_cast<U64>(pt.y)}) for(int b=0;b<8;++b) {
            hash^=(value>>(8*b))&255; hash*=1099511628211ULL;
        }
    }}
    return std::make_tuple(polys.size(),rings,verts,hash);
}
static std::string json_string(const std::string& s) {
    std::ostringstream o; o<<'"'; for(unsigned char c:s) {
        if(c=='"'||c=='\\') o<<'\\'<<c;
        else if(c<32) { const char* hex="0123456789abcdef"; o<<"\\u00"<<hex[c>>4]<<hex[c&15]; }
        else o<<c;
    } o<<'"'; return o.str();
}
int main(int argc,char** argv) { try {
    require(argc>1,"usage: driver corpus.cases ...");
    for(int f=1;f<argc;++f) {
        std::ifstream stream(argv[f]); require(stream.good(),"cannot read corpus"); Tokens t; std::string line;
        while(std::getline(stream,line)) {
            line=line.substr(0,line.find('#')); std::istringstream in(line); std::string token;
            while(in>>token) t.values.push_back(token);
        }
        size_t index=0;
        while(t.pos<t.values.size()) {
            require(t.next()=="case","expected case"); Case c; c.site=t.next();c.layer=t.signed_number();c.opcode=t.number();
            if(t.pos<t.values.size() && !t.values[t.pos].empty() && std::isdigit(static_cast<unsigned char>(t.values[t.pos][0]))) t.number();
            bool subj=false,clip=false;
            for(;;) { auto record=t.next();
                if(record=="arg") { auto id=t.number(); auto value=t.number(); require(c.args.emplace(id,value).second,"duplicate arg"); }
                else if(record=="subj") { require(!subj,"duplicate subj");subj=true;c.subj=t.paths(); }
                else if(record=="clip") { require(!clip,"duplicate clip");clip=true;c.clip=t.paths(); }
                else if(record=="end") break;
                else throw std::runtime_error("unknown record");
            }
            require(subj && clip==(c.opcode!=0),"incorrect geometry records"); c.validate();
            size_t input_verts=c.input_verts();
            PolyTree64 warm; c.run(warm); auto expected=summary(warm); warm.Clear();
            U64 alloc_count=0, alloc_bytes=0;
            {
                reset_counters();
                PolyTree64 output; c.run(output);
                auto s=summary(output); require(s==expected,"nondeterministic output");
                alloc_count=g_alloc_count; alloc_bytes=g_alloc_bytes;
            }
            std::vector<U64> times;
            for(int r=0;r<5;++r) {
                // Match Rust: include tree construction, but not destruction.
                auto start=std::chrono::steady_clock::now(); PolyTree64 output; c.run(output);
                auto finish=std::chrono::steady_clock::now(); times.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(finish-start).count());
                require(summary(output)==expected,"nondeterministic output");
            }
            std::ostringstream hash; hash<<std::hex; hash.width(16);hash.fill('0');hash<<std::get<3>(expected);
            std::sort(times.begin(),times.end());
            std::cout<<"{\"file\":"<<json_string(argv[f])<<",\"case_index\":"<<index++<<",\"site\":"<<json_string(c.site)<<",\"layer\":"<<c.layer<<",\"opcode\":"<<c.opcode<<",\"in_verts\":"<<input_verts<<",\"repeat_times_ns\":[";
            for(size_t r=0;r<times.size();++r) { if(r) std::cout<<',';std::cout<<times[r]; }
            std::cout<<"],\"out_paths\":"<<std::get<0>(expected)<<",\"out_rings\":"<<std::get<1>(expected)<<",\"out_verts\":"<<std::get<2>(expected)<<",\"out_hash\":\""<<hash.str()<<"\",\"alloc_count\":"<<alloc_count<<",\"alloc_bytes\":"<<alloc_bytes<<"}\n";
        }
    }
} catch(const std::exception& e) { std::cerr<<e.what()<<'\n';return 1; } }