// main.cpp - entry point + client / mapwriter / mapreader / whoami2 modes (M0-T4 PoC).
// Modes:
//   server    --pipe N [--sddl SDDL] [--allow SID,SID] [--max K] [--secs T]
//   client    --pipe N
//   mapwriter --name N [--global|--local] [--sddl SDDL] [--secs T]
//   mapreader --name N [--global|--local]
//   whoami2
#include "common.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int RunClient(const std::vector<std::string>& args) {
    const std::string name = ArgGet(args, "--pipe");
    if (name.empty()) {
        Result("CLIENT-ERR", "missing --pipe");
        return 2;
    }
    const std::string full = "\\\\.\\pipe\\" + name;
    HANDLE h = CreateFileA(full.c_str(), GENERIC_READ | GENERIC_WRITE, 0,
                           NULL, OPEN_EXISTING, 0, NULL);
    if (h == INVALID_HANDLE_VALUE) {
        Result("CLIENT", "CreateFile FAILED gle=%lu (gle=5 means OS-level DACL rejection)",
               GetLastError());
        return 1;
    }
    Result("CLIENT", "connected pid=%lu sid=%s session=%lu",
           GetCurrentProcessId(), GetTokenSidString().c_str(),
           (unsigned long)CurrentSessionId());
    const char* msg = "hello-from-client";
    DWORD wrote = 0;
    if (!WriteFile(h, msg, (DWORD)strlen(msg), &wrote, NULL)) {
        Result("CLIENT-ERR", "WriteFile gle=%lu", GetLastError());
        CloseHandle(h);
        return 2;
    }
    char buf[256] = {0};
    DWORD got = 0;
    const BOOL ok = ReadFile(h, buf, sizeof(buf) - 1, &got, NULL);
    CloseHandle(h);
    if (!ok) {
        Result("CLIENT-ERR", "ReadFile gle=%lu", GetLastError());
        return 2;
    }
    Result("CLIENT", "server-response=%s", buf);
    return 0;
}

static int RunMapWriter(const std::vector<std::string>& args) {
    const std::string name = ArgGet(args, "--name");
    const bool global = ArgHas(args, "--global");
    const std::string sddl = ArgGet(args, "--sddl");
    const int secs = atoi(ArgGet(args, "--secs", "60").c_str());
    if (name.empty()) {
        Result("MAPWRITER-ERR", "missing --name");
        return 2;
    }

    SECURITY_ATTRIBUTES sa;
    ZeroMemory(&sa, sizeof(sa));
    sa.nLength = sizeof(sa);
    PSECURITY_DESCRIPTOR psd = NULL;
    if (!sddl.empty()) {
        if (!ConvertStringSecurityDescriptorToSecurityDescriptorA(
                sddl.c_str(), SDDL_REVISION_1, &psd, NULL)) {
            Result("MAPWRITER-ERR", "ConvertStringSecurityDescriptor failed gle=%lu", GetLastError());
            return 2;
        }
        sa.lpSecurityDescriptor = psd;
    }

    const std::string full = (global ? "Global\\" : "Local\\") + name;
    HANDLE h = CreateFileMappingA(INVALID_HANDLE_VALUE, &sa, PAGE_READWRITE, 0, 4096, full.c_str());
    if (psd) LocalFree(psd);
    if (!h) {
        Result("MAPWRITER", "CreateFileMapping FAILED gle=%lu name=%s (gle=5 may mean missing SeCreateGlobalPrivilege)",
               GetLastError(), full.c_str());
        return 1;
    }
    void* view = MapViewOfFile(h, FILE_MAP_WRITE, 0, 0, 4096);
    if (!view) {
        Result("MAPWRITER", "MapViewOfFile FAILED gle=%lu", GetLastError());
        CloseHandle(h);
        return 1;
    }
    char payload[256] = {0};
    sprintf(payload,
            "M0T4|writerPid=%lu|writerSid=%s|writerSession=%lu|tick=%llu|msg=hello-across-sessions",
            GetCurrentProcessId(), GetTokenSidString().c_str(),
            (unsigned long)CurrentSessionId(), (unsigned long long)GetTickCount64());
    memcpy(view, payload, strlen(payload));
    Result("MAPWRITER", "created name=%s global=%d payload=%s (alive %ds)",
           full.c_str(), global ? 1 : 0, payload, secs);
    Sleep((DWORD)secs * 1000);
    Result("MAPWRITER", "done name=%s", full.c_str());
    CloseHandle(h);
    return 0;
}

static int RunMapReader(const std::vector<std::string>& args) {
    const std::string name = ArgGet(args, "--name");
    const bool global = ArgHas(args, "--global");
    if (name.empty()) {
        Result("MAPREADER-ERR", "missing --name");
        return 2;
    }
    const std::string full = (global ? "Global\\" : "Local\\") + name;
    HANDLE h = OpenFileMappingA(FILE_MAP_READ, FALSE, full.c_str());
    if (!h) {
        Result("MAPREADER", "OpenFileMapping FAILED gle=%lu name=%s readerSid=%s readerSession=%lu "
                            "(gle=2 across sessions for Local\\ = namespace isolation; gle=5 = DACL)",
               GetLastError(), full.c_str(), GetTokenSidString().c_str(),
               (unsigned long)CurrentSessionId());
        return 1;
    }
    void* view = MapViewOfFile(h, FILE_MAP_READ, 0, 0, 4096);
    if (!view) {
        Result("MAPREADER", "MapViewOfFile FAILED gle=%lu", GetLastError());
        CloseHandle(h);
        return 1;
    }
    char payload[257] = {0};
    memcpy(payload, view, 256);
    UnmapViewOfFile(view);
    CloseHandle(h);
    Result("MAPREADER", "OPEN-OK name=%s readerSid=%s readerSession=%lu payload=%s",
           full.c_str(), GetTokenSidString().c_str(),
           (unsigned long)CurrentSessionId(), payload);
    return 0;
}

static int RunWhoami2() {
    Result("WHOAMI2", "pid=%lu sid=%s session=%lu seCreateGlobal=%d seIncreaseQuota=%d",
           GetCurrentProcessId(), GetTokenSidString().c_str(),
           (unsigned long)CurrentSessionId(),
           HasPrivilege("SeCreateGlobalPrivilege") ? 1 : 0,
           HasPrivilege("SeIncreaseQuotaPrivilege") ? 1 : 0);
    return 0;
}

int main(int argc, char** argv) {
    const std::string mode = (argc > 1) ? argv[1] : "";
    std::vector<std::string> args(argv + 2, argv + argc);
    if (mode == "server") return RunServer(args);
    if (mode == "client") return RunClient(args);
    if (mode == "mapwriter") return RunMapWriter(args);
    if (mode == "mapreader") return RunMapReader(args);
    if (mode == "whoami2") return RunWhoami2();
    Result("POC-ERR", "unknown mode '%s' (server|client|mapwriter|mapreader|whoami2)", mode.c_str());
    return 2;
}
