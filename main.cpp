#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0601
#endif
#include "portable.h"
#include "sdk.h"
#include <atomic>
#include <thread>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <vector>
#include <string>

#ifdef _MSC_VER
#pragma comment(linker, "/MANIFESTUAC:level='requireAdministrator'")
#pragma comment(lib, "psapi.lib")
#endif

using namespace std::chrono;

static std::atomic<bool> g_run{ true };
static std::atomic<bool> g_esp{ true };
static std::atomic<bool> g_box{ true };
static std::atomic<bool> g_health{ true };
static std::atomic<bool> g_names{ true };
static std::atomic<bool> g_dist{ true };
static std::atomic<bool> g_snap{ false };
static std::atomic<bool> g_norecoil{ true };
static std::atomic<bool> g_cross{ true };
static std::atomic<bool> g_menu{ true };
static std::atomic<bool> g_attached{ false };
static std::atomic<bool> g_resolved{ false };
static std::atomic<bool> g_writable{ false };
static std::atomic<int>  g_ent{ 0 };
static std::atomic<int>  g_ply{ 0 };

static HWND     g_overlay = nullptr;
static int      g_w = 0, g_h = 0;
static COLORREF KEY = RGB(255, 0, 255);
static bool     g_priv = false;
static std::string g_rlog;
static char     g_stage[96] = "start";

static HFONT g_font = nullptr, g_fontB = nullptr, g_fontS = nullptr;

static const COLORREF C_BG     = RGB(14, 16, 22);
static const COLORREF C_BAR    = RGB(24, 30, 34);
static const COLORREF C_BORDER = RGB(64, 210, 168);
static const COLORREF C_ACC    = RGB(64, 210, 168);
static const COLORREF C_TEXT   = RGB(228, 232, 240);
static const COLORREF C_DIM    = RGB(132, 140, 154);
static const COLORREF C_OK     = RGB(90, 225, 130);
static const COLORREF C_BAD    = RGB(240, 95, 95);
static const COLORREF C_ROW    = RGB(30, 35, 44);

static void FillC(HDC dc, int x, int y, int w, int h, COLORREF c)
{
    RECT r{ x, y, x + w, y + h };
    HBRUSH b = CreateSolidBrush(c);
    FillRect(dc, &r, b);
    DeleteObject(b);
}

static void FrameC(HDC dc, int x, int y, int w, int h, COLORREF c, int t = 1)
{
    HPEN p = CreatePen(PS_SOLID, t, c);
    HGDIOBJ op = SelectObject(dc, p);
    HGDIOBJ ob = SelectObject(dc, GetStockObject(NULL_BRUSH));
    Rectangle(dc, x, y, x + w, y + h);
    SelectObject(dc, ob);
    SelectObject(dc, op);
    DeleteObject(p);
}

static void TextC(HDC dc, int x, int y, const char* s, COLORREF c, HFONT f = nullptr, bool sh = true)
{
    HGDIOBJ of = f ? SelectObject(dc, f) : nullptr;
    if (sh) { SetTextColor(dc, RGB(0, 0, 0)); TextOutA(dc, x + 1, y + 1, s, (int)std::strlen(s)); }
    SetTextColor(dc, c);
    TextOutA(dc, x, y, s, (int)std::strlen(s));
    if (of) SelectObject(dc, of);
}

static bool IsAdmin()
{
    BOOL admin = FALSE;
    SID_IDENTIFIER_AUTHORITY nt = SECURITY_NT_AUTHORITY;
    PSID grp = nullptr;
    if (AllocateAndInitializeSid(&nt, 2, SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &grp))
    {
        CheckTokenMembership(nullptr, grp, &admin);
        FreeSid(grp);
    }
    return admin != FALSE;
}

static bool RelaunchElevated()
{
    char self[MAX_PATH]{};
    GetModuleFileNameA(nullptr, self, MAX_PATH);
    std::string params;
    for (int i = 1; i < __argc; ++i) { params += "\""; params += __argv[i]; params += "\" "; }

    SHELLEXECUTEINFOA sei{ sizeof(sei) };
    sei.fMask  = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = "runas";
    sei.lpFile = self;
    sei.lpParameters = params.empty() ? nullptr : params.c_str();
    sei.nShow  = SW_SHOWNORMAL;
    return ShellExecuteExA(&sei) != FALSE;
}static const char* ILName(int lvl)
{
    switch (lvl)
    {
        case 0x1000: return "LOW";
        case 0x2000: return "MEDIUM";
        case 0x3000: return "HIGH (ADMIN)";
        case 0x4000: return "SYSTEM";
        default:     return "n/a";
    }
}

static int IntegrityLevel(DWORD pid)
{
    if (!pid) return -1;
    HANDLE h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (!h) h = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);
    if (!h) return -1;
    int lvl = -1;
    HANDLE tok = nullptr;
    if (OpenProcessToken(h, TOKEN_QUERY, &tok))
    {
        DWORD sz = 0;
        GetTokenInformation(tok, TokenIntegrityLevel, nullptr, 0, &sz);
        std::vector<BYTE> b(sz ? sz : 64);
        if (GetTokenInformation(tok, TokenIntegrityLevel, b.data(), (DWORD)b.size(), &sz))
        {
            auto* t = (TOKEN_MANDATORY_LABEL*)b.data();
            lvl = (int)*GetSidSubAuthority(t->Label.Sid,
                       (DWORD)(*GetSidSubAuthorityCount(t->Label.Sid) - 1));
        }
        CloseHandle(tok);
    }
    CloseHandle(h);
    return lvl;
}

static void ProbeOpen(DWORD pid)
{
    struct T { const char* n; DWORD r; };
    static const T tries[] = {
        { "ALL_ACCESS",            PROCESS_ALL_ACCESS },
        { "RW vm_rw|op|query",     PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION },
        { "RO vm_read|query_inf",  PROCESS_VM_READ | PROCESS_QUERY_INFORMATION },
    };
    for (auto& t : tries)
    {
        SetLastError(0);
        HANDLE h = OpenProcess(t.r, FALSE, pid);
        std::printf("[vrx]   probe %-22s %s (err %lu)\n", t.n, h ? "OK" : "FAIL", GetLastError());
        if (h) CloseHandle(h);
    }
    SetLastError(0);
    HANDLE s = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
    std::printf("[vrx]   probe snapshot %s (err %lu)\n",
                s != INVALID_HANDLE_VALUE ? "OK" : "FAIL", GetLastError());
    if (s != INVALID_HANDLE_VALUE) CloseHandle(s);
}

static void DumpDiag(DWORD pid, DWORD err, const char* stage)
{
    std::printf("[vrx] --- diag: err %lu, stage %s, bitness %d ---\n", err, stage, (int)(sizeof(void*) * 8));
    std::printf("[vrx] vrx integrity  : %s\n", ILName(IntegrityLevel(GetCurrentProcessId())));
    std::printf("[vrx] Rust integrity : %s\n", ILName(IntegrityLevel(pid)));
    std::printf("[vrx] SeDebug        : %s\n", g_priv ? "UKLJUCEN" : "NEMA");
    if (pid) ProbeOpen(pid);
}

static void Hotkeys()
{
    static bool last[16] = {};
    struct B { int vk; std::atomic<bool>* t; };
    static const B b[] = {
        { VK_F1, &g_esp }, { VK_F2, &g_box }, { VK_F3, &g_health }, { VK_F4, &g_names },
        { VK_F5, &g_dist }, { VK_F6, &g_snap }, { VK_F7, &g_norecoil }, { VK_F8, &g_cross },
        { VK_INSERT, &g_menu },
    };
    const int n = (int)(sizeof(b) / sizeof(b[0]));
    for (int i = 0; i < n; ++i)
    {
        const bool down = (GetAsyncKeyState(b[i].vk) & 0x8000) != 0;
        if (down && !last[i]) b[i].t->store(!b[i].t->load());
        last[i] = down;
    }
    static bool eLast = false;
    const bool eDown = (GetAsyncKeyState(VK_END) & 0x8000) != 0;
    if (eDown && !eLast) g_run = false;
    eLast = eDown;
}static LRESULT CALLBACK WndProc(HWND h, UINT m, WPARAM w, LPARAM l)
{
    if (m == WM_DESTROY) { PostQuitMessage(0); return 0; }
    return DefWindowProcA(h, m, w, l);
}

static void CreateOverlay()
{
    WNDCLASSA wc{};
    wc.lpfnWndProc   = WndProc;
    wc.hInstance     = GetModuleHandleA(nullptr);
    wc.lpszClassName = "vrxOverlay";
    wc.hCursor       = LoadCursorA(nullptr, IDC_ARROW);
    RegisterClassA(&wc);

    g_w = GetSystemMetrics(SM_CXSCREEN);
    g_h = GetSystemMetrics(SM_CYSCREEN);

    g_overlay = CreateWindowExA(
        WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
        "vrxOverlay", "vrx", WS_POPUP, 0, 0, g_w, g_h, nullptr, nullptr, wc.hInstance, nullptr);

    SetLayeredWindowAttributes(g_overlay, KEY, 0, LWA_COLORKEY);
    ShowWindow(g_overlay, SW_SHOW);

    g_font  = CreateFontA(-13, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
                          OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH, "Segoe UI");
    g_fontB = CreateFontA(-14, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET,
                          OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH, "Segoe UI");
    g_fontS = CreateFontA(-11, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET,
                          OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH, "Segoe UI");
}

static void KeepTop()
{
    if (!g_overlay) return;
    const int w = GetSystemMetrics(SM_CXSCREEN);
    const int h = GetSystemMetrics(SM_CYSCREEN);
    if (w != g_w || h != g_h) { g_w = w; g_h = h; }
    SetWindowPos(g_overlay, HWND_TOPMOST, 0, 0, g_w, g_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
}

static void DrawLineC(HDC dc, int x1, int y1, int x2, int y2, COLORREF c, int t = 1)
{
    HPEN p = CreatePen(PS_SOLID, t, c);
    HGDIOBJ op = SelectObject(dc, p);
    MoveToEx(dc, x1, y1, nullptr);
    LineTo(dc, x2, y2);
    SelectObject(dc, op);
    DeleteObject(p);
}

static void DrawCorners(HDC dc, int x, int y, int w, int h, COLORREF c, int L)
{
    DrawLineC(dc, x, y, x + L, y, c);
    DrawLineC(dc, x, y, x, y + L, c);
    DrawLineC(dc, x + w, y, x + w - L, y, c);
    DrawLineC(dc, x + w, y, x + w, y + L, c);
    DrawLineC(dc, x, y + h, x + L, y + h, c);
    DrawLineC(dc, x, y + h, x, y + h - L, c);
    DrawLineC(dc, x + w, y + h, x + w - L, y + h, c);
    DrawLineC(dc, x + w, y + h, x + w, y + h - L, c);
}

static COLORREF DistCol(float d)
{
    if (d < 50.f)  return RGB(90, 255, 130);
    if (d < 120.f) return RGB(230, 240, 90);
    if (d < 250.f) return RGB(255, 170, 60);
    return RGB(255, 100, 100);
}

static void DrawMenu(HDC dc)
{
    struct Row { const char* n; const char* k; bool v; bool hdr; };
    const Row rows[] = {
        { "ESP",        "F1",  g_esp.load(),      true  },
        { "  Box",      "F2",  g_box.load(),      false },
        { "  Health",   "F3",  g_health.load(),   false },
        { "  Ime",      "F4",  g_names.load(),    false },
        { "  Distanca", "F5",  g_dist.load(),     false },
        { "  Snapline", "F6",  g_snap.load(),     false },
        { "No recoil",  "F7",  g_norecoil.load(), true  },
        { "Crosshair",  "F8",  g_cross.load(),    false },
        { "Menu",       "INS", g_menu.load(),     false },
    };
    const int n = (int)(sizeof(rows) / sizeof(rows[0]));
    const int px = 20, py = 20, pw = 300, rowH = 24, headH = 36;
    const int ph = headH + n * rowH + 44;

    FillC(dc, px + 5, py + 5, pw, ph, RGB(8, 9, 13));
    FillC(dc, px, py, pw, ph, C_BG);
    FrameC(dc, px, py, pw, ph, C_BORDER, 1);
    FillC(dc, px + 1, py + 1, pw - 2, headH - 1, C_BAR);
    FrameC(dc, px + 1, py + 1, pw - 2, headH - 1, C_BORDER, 1);

    TextC(dc, px + 14, py + 9, "v r x", C_ACC, g_fontB);
    TextC(dc, px + 66, py + 12, "rust external", C_DIM, g_fontS);
    TextC(dc, px + pw - 62, py + 12, "INSERT", C_DIM, g_fontS);

    int y = py + headH + 6;
    for (int i = 0; i < n; ++i)
    {
        if (rows[i].v) FillC(dc, px + 8, y - 3, pw - 16, rowH - 2, C_ROW);
        const int cx = px + 16, cy = y + 3;
        FrameC(dc, cx, cy, 13, 13, rows[i].v ? C_OK : C_DIM, 1);
        if (rows[i].v) FillC(dc, cx + 4, cy + 4, 5, 5, C_OK);
        TextC(dc, px + 40, y + 2, rows[i].n, rows[i].v ? C_TEXT : C_DIM, rows[i].hdr ? g_fontB : g_font);
        TextC(dc, px + pw - 52, y + 4, rows[i].k, C_DIM, g_fontS);
        y += rowH;
    }

    char foot[180];
    std::snprintf(foot, sizeof(foot), "END = izlaz   |   attach %s   resolve %s",
                  g_attached ? "OK" : "-", g_resolved ? "OK" : "-");
    TextC(dc, px + 14, y + 6, foot, C_DIM, g_fontS);
}static void Render()
{
    HDC hdc = GetDC(g_overlay);
    static HDC mem = nullptr;
    static HBITMAP bmp = nullptr;
    static int bw = 0, bh = 0;
    if (!mem || bw != g_w || bh != g_h)
    {
        if (bmp) DeleteObject(bmp);
        if (mem) DeleteDC(mem);
        mem = CreateCompatibleDC(hdc);
        bmp = CreateCompatibleBitmap(hdc, g_w, g_h);
        SelectObject(mem, bmp);
        bw = g_w; bh = g_h;
    }

    RECT full{ 0, 0, g_w, g_h };
    HBRUSH bg = CreateSolidBrush(KEY);
    FillRect(mem, &full, bg);
    DeleteObject(bg);
    SetBkMode(mem, TRANSPARENT);

    if (g_cross)
    {
        const int cx = g_w / 2, cy = g_h / 2;
        DrawLineC(mem, cx - 9, cy, cx - 2, cy, RGB(255, 255, 255));
        DrawLineC(mem, cx + 2, cy, cx + 9, cy, RGB(255, 255, 255));
        DrawLineC(mem, cx, cy - 9, cx, cy - 2, RGB(255, 255, 255));
        DrawLineC(mem, cx, cy + 2, cx, cy + 9, RGB(255, 255, 255));
    }

    int players = 0;

    if (g_esp && g_resolved)
    {
        Matrix4x4 vp{};
        float fov = 60.f;
        if (game::ViewProjection(vp, fov))
        {
            const Vector3 eye = game::LocalEyes();
            auto list = game::GetPlayers(eye, {});
            for (auto& p : list)
            {
                if (p.isLocal) continue;
                Vector2 fb{}, hb{};
                if (!game::WorldToScreen(p.feet, vp, (float)g_w, (float)g_h, fb)) continue;
                if (!game::WorldToScreen(p.head, vp, (float)g_w, (float)g_h, hb)) continue;

                const float hgt = fb.y - hb.y;
                if (hgt < 4.f || hgt > (float)g_h * 1.5f) continue;
                const int w = (int)(hgt * 0.55f);
                const int x = (int)(hb.x - w * 0.5f);
                const int y = (int)hb.y;
                const int h = (int)hgt;
                const COLORREF col = DistCol(p.distance);
                ++players;

                if (g_snap) DrawLineC(mem, g_w / 2, g_h - 1, x + w / 2, y + h, RGB(120, 200, 255));
                if (g_box)
                {
                    FrameC(mem, x, y, w, h, col, 1);
                    DrawCorners(mem, x - 2, y - 2, w + 4, h + 4, col, 7);
                }

                if (g_health && p.health > 0.f && p.health <= 100.f)
                {
                    const float f = p.health / 100.f;
                    FillC(mem, x - 8, y - 1, 6, h + 2, RGB(0, 0, 0));
                    FillC(mem, x - 7, y + (int)(h * (1.f - f)), 4, (int)(h * f),
                          RGB((int)(255 * (1.f - f)), (int)(255 * f), 40));
                }

                char txt[128];
                if (g_names)
                {
                    std::snprintf(txt, sizeof(txt), "%s", p.name.c_str());
                    TextC(mem, x + w / 2 - (int)std::strlen(txt) * 3, y - 18, txt, col, g_font);
                }
                if (g_dist)
                {
                    std::snprintf(txt, sizeof(txt), "%.0f m", p.distance);
                    TextC(mem, x + w / 2 - (int)std::strlen(txt) * 3, y + h + 3, txt, C_TEXT, g_fontS);
                }
            }
        }
    }
    g_ply = players;

    if (g_menu) DrawMenu(mem);

    char st[220];
    std::snprintf(st, sizeof(st), "%s  |  rw=%s  ents=%d  players=%d",
                  g_stage, g_writable ? "yes" : "no", g_ent.load(), g_ply.load());
    TextC(mem, 22, g_h - 26, st, g_attached ? C_OK : C_DIM, g_fontS);

    if (!g_rlog.empty())
    {
        int y = 24;
        TextC(mem, g_w - 520, y, "resolve log:", C_BAD, g_fontB); y += 18;
        size_t i = 0;
        while (i < g_rlog.size() && y < g_h - 40)
        {
            const size_t j = g_rlog.find('\n', i);
            const std::string line = g_rlog.substr(i, (j == std::string::npos ? g_rlog.size() : j) - i);
            if (!line.empty()) TextC(mem, g_w - 520, y, line.c_str(), C_BAD, g_fontS);
            y += 15;
            if (j == std::string::npos) break;
            i = j + 1;
        }
    }

    BitBlt(hdc, 0, 0, g_w, g_h, mem, 0, 0, SRCCOPY);
    ReleaseDC(g_overlay, hdc);
}

static void NoRecoil()
{
    if (!mem.writable) return;

    static uintptr_t kProj = 0;
    static int32_t yMin = -2, yMax = -2, pMin = -2, pMax = -2;
    if (!kProj) kProj = il2c::FindClass("BaseProjectile");
    if (!kProj) return;

    for (uintptr_t e : game::Entities())
    {
        if (mem.Read<uintptr_t>(e) != kProj) continue;

        if (O.fAimSway >= 0)      mem.Write<float>(e + O.fAimSway, 0.f);
        if (O.fAimSwaySpeed >= 0) mem.Write<float>(e + O.fAimSwaySpeed, 0.f);

        if (O.fRecoilProps < 0) continue;
        uintptr_t rp = mem.Read<uintptr_t>(e + O.fRecoilProps);
        if (rp < 0x10000) continue;
        uintptr_t k = mem.Read<uintptr_t>(rp);
        if (k < 0x10000) continue;

        if (yMin == -2)
        {
            yMin = il2c::FindFieldAny(k, { "recoilYawMin", "yawMin" });
            yMax = il2c::FindFieldAny(k, { "recoilYawMax", "yawMax" });
            pMin = il2c::FindFieldAny(k, { "recoilPitchMin", "pitchMin" });
            pMax = il2c::FindFieldAny(k, { "recoilPitchMax", "pitchMax" });
        }
        if (yMin >= 0) mem.Write<float>(rp + yMin, 0.f);
        if (yMax >= 0) mem.Write<float>(rp + yMax, 0.f);
        if (pMin >= 0) mem.Write<float>(rp + pMin, 0.f);
        if (pMax >= 0) mem.Write<float>(rp + pMax, 0.f);
    }
}static bool ProcessRunning(const wchar_t* name)
{
    HANDLE s = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (s == INVALID_HANDLE_VALUE) return false;
    PROCESSENTRY32W pe{}; pe.dwSize = sizeof(pe);
    bool found = false;
    if (Process32FirstW(s, &pe))
        do { if (!_wcsicmp(pe.szExeFile, name)) { found = true; break; } }
        while (Process32NextW(s, &pe));
    CloseHandle(s);
    return found;
}

static void Worker()
{
    std::string log;
    bool saidWaiting = false;

    while (g_run)
    {
        Hotkeys();
        KeepTop();

        if (!mem.Ok() || !mem.Alive())
        {
            if (g_attached) { mem.Detach(); g_attached = false; g_resolved = false; g_writable = false; }

            if (!ProcessRunning(L"RustClient.exe"))
            {
                std::snprintf(g_stage, sizeof(g_stage), "cekam RustClient.exe");
                if (!saidWaiting) { std::printf("[vrx] cekam RustClient.exe ...\n"); saidWaiting = true; }
                Render();
                std::this_thread::sleep_for(milliseconds(300));
                continue;
            }
            saidWaiting = false;

            std::snprintf(g_stage, sizeof(g_stage), "attach...");
            bool ok = mem.Attach(L"RustClient.exe", L"GameAssembly.dll", Memory::Access::ReadWrite);
            if (!ok)
            {
                const DWORD e1 = mem.lastErr ? mem.lastErr : GetLastError();
                const std::string stg = mem.lastStage;
                const DWORD tpid = mem.pid;
                mem.Detach();
                std::snprintf(g_stage, sizeof(g_stage), "attach FAILED err %lu (%s)", e1, stg.c_str());
                std::printf("[vrx] attach FAILED err %lu stage=%s\n", e1, stg.c_str());
                DumpDiag(tpid, e1, stg.c_str());
                ok = mem.Attach(L"RustClient.exe", L"GameAssembly.dll", Memory::Access::ReadOnly);
            }

            if (ok)
            {
                g_attached = true;
                g_writable = mem.writable;
                std::printf("[vrx] attach OK pid %lu %s via %s\n", mem.CurrentPid(),
                            mem.writable ? "RW" : "RO", mem.foundVia.c_str());
                if (game::Resolve(log))
                {
                    g_resolved = true;
                    g_rlog.clear();
                    std::snprintf(g_stage, sizeof(g_stage), "attach %s, resolved", mem.writable ? "RW" : "RO");
                    std::printf("[vrx] resolved OK\n");
                }
                else
                {
                    g_rlog = log;
                    std::snprintf(g_stage, sizeof(g_stage), "resolve FAILED");
                    std::printf("[vrx] resolve FAILED:\n%s\n", log.c_str());
                    mem.Detach();
                    g_attached = false;
                }
            }
            Render();
            std::this_thread::sleep_for(milliseconds(500));
            continue;
        }

        if (g_norecoil) NoRecoil();
        g_ent = (int)game::Entities().size();
        Render();
        std::this_thread::sleep_for(milliseconds(1));
    }
}

int main()
{
    SetConsoleTitleA("vrx - rust external");
    std::printf("vrx starting. admin=%s bitness=%d-bit\n",
                IsAdmin() ? "yes" : "no", (int)(sizeof(void*) * 8));
    if (sizeof(void*) == 4)
        std::printf("vrx: PAZI - 32-bit build!\n");
    std::printf("INSERT = menu | F1 ESP | F2 box | F3 health | F4 ime | F5 distanca\n");
    std::printf("F6 snapline | F7 norecoil | F8 crosshair | END = izlaz\n");
    std::printf("Ako ne vidis overlay u igri -> Rust stavi na BORDERLESS (Fullscreen OFF).\n");

    if (!IsAdmin())
    {
        std::printf("vrx: dizem UAC...\n");
        if (RelaunchElevated()) return 0;
    }

    g_priv = EnableDebugPriv();
    std::printf("vrx: SeDebugPrivilege = %s\n", g_priv ? "UKLJUCEN" : "NEMA");

    portable::Init();
    if (portable::RelocateIfNeeded()) return 0;

    CreateOverlay();
    std::thread w(Worker);

    MSG msg;
    while (g_run)
    {
        while (PeekMessageA(&msg, nullptr, 0, 0, PM_REMOVE))
        {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
        Sleep(5);
    }

    g_run = false;
    w.join();
    portable::Wipe(true);
    return 0;
}