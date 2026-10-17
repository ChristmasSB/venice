#pragma once
#include <Windows.h>
#include <string>
#include <cstdio>

namespace portable {

inline std::string& RunPath() { static std::string s; return s; }
inline std::string& RunDir()  { static std::string s; return s; }

inline std::string TempExe()
{
    char t[MAX_PATH]{};
    GetTempPathA(MAX_PATH, t);
    return std::string(t) + "vrx_run.exe";
}

inline void Init()
{
    char self[MAX_PATH]{};
    GetModuleFileNameA(nullptr, self, MAX_PATH);
    RunPath() = self;
    std::string s = self;
    const size_t p = s.find_last_of("\\/");
    RunDir() = (p == std::string::npos) ? "." : s.substr(0, p);
}

// ako se ne vrtimo iz %TEMP%, kopiraj se tamo i pokreni odande (da USB moze van)
inline bool RelocateIfNeeded()
{
    char t[MAX_PATH]{};
    GetTempPathA(MAX_PATH, t);

    std::string self = RunPath(), tmp = t;
    for (auto& c : self) c = (char)tolower((unsigned char)c);
    for (auto& c : tmp)  c = (char)tolower((unsigned char)c);
    if (self.rfind(tmp, 0) == 0) return false;

    const std::string target = TempExe();
    if (!CopyFileA(RunPath().c_str(), target.c_str(), FALSE)) return false;

    std::string cmd = "\"" + target + "\"";
    STARTUPINFOA si{ sizeof(si) };
    PROCESS_INFORMATION pi{};
    if (!CreateProcessA(nullptr, (char*)cmd.c_str(), nullptr, nullptr, FALSE,
                        CREATE_NEW_CONSOLE, nullptr, nullptr, &si, &pi))
        return false;
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    std::printf("[portable] kopiran u %s i pokrenut odande (USB moze van)\n", target.c_str());
    return true;
}

inline void Wipe(bool)
{
    const std::string target = TempExe();
    if (GetFileAttributesA(target.c_str()) == INVALID_FILE_ATTRIBUTES) return;
    std::string cmd = "cmd /c ping -n 2 127.0.0.1 >nul & del /f /q \"" + target + "\"";
    STARTUPINFOA si{ sizeof(si) };
    PROCESS_INFORMATION pi{};
    if (CreateProcessA(nullptr, (char*)cmd.c_str(), nullptr, nullptr, FALSE,
                       CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi))
    { CloseHandle(pi.hThread); CloseHandle(pi.hProcess); }
}
}