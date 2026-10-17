#pragma once
#include "memory.h"
#include <cmath>
#include <cstring>
#include <cstdio>
#include <string>
#include <vector>
#include <initializer_list>
#include <algorithm>
#include <chrono>

struct Vector2 { float x = 0.f, y = 0.f; };
struct Vector3 { float x = 0.f, y = 0.f, z = 0.f; };
struct Quat    { float x = 0.f, y = 0.f, z = 0.f, w = 1.f; };
struct Matrix4x4 { float m[16] = {}; };

inline Vector3 V3Sub(const Vector3& a, const Vector3& b) { return { a.x - b.x, a.y - b.y, a.z - b.z }; }
inline float   V3Dot(const Vector3& a, const Vector3& b) { return a.x * b.x + a.y * b.y + a.z * b.z; }
inline Vector3 V3Cross(const Vector3& a, const Vector3& b)
{ return { a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x }; }
inline Vector3 V3Norm(const Vector3& a)
{
    const float l = std::sqrt(V3Dot(a, a));
    return l > 0.0001f ? Vector3{ a.x / l, a.y / l, a.z / l } : Vector3{ 0.f, 0.f, 1.f };
}
inline Vector3 QuatForward(const Quat& q)
{
    return { 2.f * (q.x * q.z + q.w * q.y),
             2.f * (q.y * q.z - q.w * q.x),
             1.f - 2.f * (q.x * q.x + q.y * q.y) };
}namespace game {

inline bool Refresh(bool force = false)
{
    if (!g_ready || !mem.hProc) return false;

    static auto last = std::chrono::steady_clock::now();
    const auto now = std::chrono::steady_clock::now();
    if (!force && std::chrono::duration_cast<std::chrono::milliseconds>(now - last).count() < 2500) return false;
    last = now;

    std::vector<uintptr_t> found;
    found.reserve(512);

    const size_t CH = 8u << 20;
    static std::vector<uint8_t> buf;
    buf.resize(CH);

    uintptr_t a = 0;
    MEMORY_BASIC_INFORMATION mbi{};
    while (VirtualQueryEx(mem.hProc, (LPCVOID)a, &mbi, sizeof(mbi)) == sizeof(mbi))
    {
        const uintptr_t b = (uintptr_t)mbi.BaseAddress;
        const uintptr_t nxt = b + mbi.RegionSize;
        if (nxt <= a) break;
        a = nxt;

        if (mbi.State != MEM_COMMIT) continue;
        if (mbi.Type == MEM_IMAGE) continue;
        if (mbi.Protect & PAGE_GUARD) continue;
        const DWORD pr = mbi.Protect & 0xFF;
        if (pr != PAGE_READWRITE && pr != PAGE_READONLY && pr != PAGE_WRITECOPY &&
            pr != PAGE_EXECUTE_READWRITE && pr != PAGE_EXECUTE_READ && pr != PAGE_EXECUTE_WRITECOPY) continue;

        for (uintptr_t off = 0; off < mbi.RegionSize; off += CH)
        {
            const size_t n = (size_t)std::min<uintptr_t>(CH, mbi.RegionSize - off);
            if (!mem.ReadRaw(b + off, buf.data(), n)) continue;
            for (size_t i = 0; i + 8 <= n; i += 8)
            {
                uintptr_t v = 0;
                std::memcpy(&v, &buf[i], 8);
                if (v == kBasePlayer || (kBaseProjectile && v == kBaseProjectile))
                    found.push_back(b + off + i);
            }
        }
    }

    entities.clear();
    entities.reserve(found.size());
    for (uintptr_t o : found)
    {
        if (mem.Read<uintptr_t>(o) == kBasePlayer)
        {
            const uintptr_t pm = mem.Read<uintptr_t>(o + O.fPlayerModel);
            if (pm < 0x10000) continue;
            const Vector3 p = mem.Read<Vector3>(pm + O.fModelPos);
            if (std::fabs(p.x) > 20000.f || std::fabs(p.z) > 20000.f) continue;
        }
        entities.push_back(o);
    }
    return true;
}

inline const std::vector<uintptr_t>& Entities()
{
    Refresh();
    return entities;
}

inline std::vector<Player> GetPlayers(const Vector3& eye, const std::vector<uintptr_t>&)
{
    Refresh();
    std::vector<Player> out;
    if (!g_ready) return out;

    for (uintptr_t o : entities)
    {
        if (mem.Read<uintptr_t>(o) != kBasePlayer) continue;

        const uintptr_t pm = mem.Read<uintptr_t>(o + O.fPlayerModel);
        if (pm < 0x10000) continue;
        const Vector3 feet = mem.Read<Vector3>(pm + O.fModelPos);
        if (feet.x == 0.f && feet.y == 0.f && feet.z == 0.f) continue;

        Player p;
        p.addr = o;
        p.feet = feet;
        p.head = { feet.x, feet.y + 1.72f, feet.z };
        if (O.fHealth >= 0) p.health = mem.Read<float>(o + O.fHealth);
        if (O.fDisplayName >= 0)
        {
            const uintptr_t s = mem.Read<uintptr_t>(o + O.fDisplayName);
            p.name = mem.ReadIl2CppString(s, 20);
        }
        if (p.name.empty()) p.name = "igrac";
        p.distance = std::sqrt(V3Dot(V3Sub(feet, eye), V3Sub(feet, eye)));
        p.isLocal = (localPlayer && o == localPlayer);
        out.push_back(p);
    }
    return out;
}

inline Vector3 LocalEyes()
{
    if (!localPlayer || O.fPlayerModel < 0) return { 0.f, 0.f, 0.f };

    if (O.fEyes >= 0 && O.fEyesPos >= 0)
    {
        const uintptr_t e = mem.Read<uintptr_t>(localPlayer + O.fEyes);
        if (e > 0x10000)
        {
            const Vector3 ep = mem.Read<Vector3>(e + O.fEyesPos);
            if (std::fabs(ep.x) < 20000.f && std::fabs(ep.z) < 20000.f && ep.y > -500.f && ep.y < 2000.f)
                return ep;
        }
    }

    const uintptr_t pm = mem.Read<uintptr_t>(localPlayer + O.fPlayerModel);
    if (pm < 0x10000) return { 0.f, 0.f, 0.f };
    const Vector3 feet = mem.Read<Vector3>(pm + O.fModelPos);
    return { feet.x, feet.y + 1.5f, feet.z };
}

inline bool LocalAim(Vector3& eye, Vector3& fwd)
{
    eye = LocalEyes();
    if (!localPlayer) return false;

    if (O.fEyes >= 0 && O.fEyesRot >= 0)
    {
        const uintptr_t e = mem.Read<uintptr_t>(localPlayer + O.fEyes);
        if (e > 0x10000)
        {
            const Quat q = mem.Read<Quat>(e + O.fEyesRot);
            const float l = std::sqrt(q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w);
            if (l > 0.5f && l < 1.5f)
            {
                const Vector3 f = QuatForward(q);
                if (std::fabs(f.z) > 0.05f || std::fabs(f.x) > 0.05f)
                { fwd = V3Norm(f); return true; }
            }
        }
    }

    if (O.fInput >= 0 && O.fBodyRot >= 0)
    {
        const uintptr_t i = mem.Read<uintptr_t>(localPlayer + O.fInput);
        if (i > 0x10000)
        {
            const float yaw = mem.Read<float>(i + O.fBodyRot);
            float pitch = 0.f;
            if (O.fViewAngles >= 0) pitch = mem.Read<Vector2>(i + O.fViewAngles).x;
            if (std::fabs(yaw) < 1000.f && std::fabs(pitch) < 90.f)
            {
                const float ry = yaw * 0.017453292f, rp = pitch * 0.017453292f;
                fwd = V3Norm({ std::sin(ry) * std::cos(rp), -std::sin(rp), std::cos(ry) * std::cos(rp) });
                return true;
            }
        }
    }
    return false;
}

inline bool ViewProjection(Matrix4x4& out, float& fov)
{
    static bool lb = false, rb = false;
    const bool l = (GetAsyncKeyState(VK_OEM_4) & 0x8000) != 0;
    const bool r = (GetAsyncKeyState(VK_OEM_6) & 0x8000) != 0;
    if (l && !lb) { g_fov -= 1.f; std::printf("[vrx] fov = %.0f\n", g_fov); }
    if (r && !rb) { g_fov += 1.f; std::printf("[vrx] fov = %.0f\n", g_fov); }
    lb = l; rb = r;

    if (localPlayer && O.fFov >= 0)
    {
        const float gf = mem.Read<float>(localPlayer + O.fFov);
        if (gf > 30.f && gf < 130.f) g_fov = gf;
    }

    Vector3 eye{}, fwd{ 0.f, 0.f, 1.f };
    if (!LocalAim(eye, fwd)) return false;

    const Vector3 right = V3Norm(V3Cross(Vector3{ 0.f, 1.f, 0.f }, fwd));
    const Vector3 up = V3Cross(fwd, right);

    out = Matrix4x4{};
    out.m[0] = right.x; out.m[1] = right.y; out.m[2] = right.z;  out.m[3]  = -V3Dot(right, eye);
    out.m[4] = up.x;    out.m[5] = up.y;    out.m[6] = up.z;     out.m[7]  = -V3Dot(up, eye);
    out.m[8] = fwd.x;   out.m[9] = fwd.y;   out.m[10] = fwd.z;   out.m[11] = -V3Dot(fwd, eye);

    const float aspect = (float)GetSystemMetrics(SM_CXSCREEN) / (float)GetSystemMetrics(SM_CYSCREEN);
    const float f = 1.f / std::tan(g_fov * 0.5f * 0.017453292f);
    for (int i = 0; i < 4; ++i) out.m[i] *= f / aspect;
    for (int i = 4; i < 8; ++i) out.m[i] *= f;
    out.m[12] = out.m[13] = out.m[14] = 0.f;
    out.m[15] = 1.f;

    fov = g_fov;
    return true;
}

inline bool WorldToScreen(const Vector3& p, const Matrix4x4& vp, float w, float h, Vector2& out)
{
    const float x = vp.m[0] * p.x + vp.m[1] * p.y + vp.m[2] * p.z + vp.m[3];
    const float y = vp.m[4] * p.x + vp.m[5] * p.y + vp.m[6] * p.z + vp.m[7];
    const float z = vp.m[8] * p.x + vp.m[9] * p.y + vp.m[10] * p.z + vp.m[11];
    if (z <= 0.01f) return false;
    out.x = (x / z + 1.f) * 0.5f * w;
    out.y = (1.f - y / z) * 0.5f * h;
    return true;
}
}