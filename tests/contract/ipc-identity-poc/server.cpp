// server.cpp - named pipe server mode with client-token identity check (M0-T4 PoC).
// Accepts one client at a time, reads the client's OS token user SID via
// GetNamedPipeClientProcessId -> OpenProcess -> OpenProcessToken -> TokenUser,
// compares it against an explicit allow list, replies, disconnects, and reports
// its own handle count per connection (leak check).
#include "common.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Reads the client process user SID behind a connected pipe instance.
static bool GetClientSidString(HANDLE pipe, std::string& sidOut, DWORD& pidOut) {
    ULONG pid = 0;
    if (!GetNamedPipeClientProcessId(pipe, &pid)) return false;
    pidOut = (DWORD)pid;
    HANDLE proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (!proc) return false;
    bool ok = false;
    HANDLE tok = NULL;
    if (OpenProcessToken(proc, TOKEN_QUERY, &tok)) {
        DWORD len = 0;
        (void)GetTokenInformation(tok, TokenUser, NULL, 0, &len);
        if (GetLastError() == ERROR_INSUFFICIENT_BUFFER && len > 0) {
            PTOKEN_USER tu = (PTOKEN_USER)malloc(len);
            if (tu && GetTokenInformation(tok, TokenUser, tu, len, &len)) {
                char* s = NULL;
                if (ConvertSidToStringSidA(tu->User.Sid, &s)) {
                    sidOut = s;
                    LocalFree(s);
                    ok = true;
                }
            }
            free(tu);
        }
        CloseHandle(tok);
    }
    CloseHandle(proc);
    return ok;
}

// Overlapped IO helper with timeout. Returns true when the transfer completed.
static bool IoWithTimeout(HANDLE h, void* buf, DWORD size, DWORD* transferred, bool write, DWORD ms) {
    OVERLAPPED ov;
    ZeroMemory(&ov, sizeof(ov));
    ov.hEvent = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (!ov.hEvent) return false;
    BOOL started = write ? WriteFile(h, buf, size, NULL, &ov)
                         : ReadFile(h, buf, size, NULL, &ov);
    if (!started && GetLastError() != ERROR_IO_PENDING) {
        CloseHandle(ov.hEvent);
        return false;
    }
    bool done = (WaitForSingleObject(ov.hEvent, ms) == WAIT_OBJECT_0);
    if (done) {
        done = GetOverlappedResult(h, &ov, transferred, FALSE) != 0;
    } else {
        CancelIo(h);
    }
    CloseHandle(ov.hEvent);
    return done;
}

int RunServer(const std::vector<std::string>& args) {
    const std::string name = ArgGet(args, "--pipe", "shutu-poc");
    const std::string sddl = ArgGet(args, "--sddl");
    const std::vector<std::string> allow = ArgList(ArgGet(args, "--allow"));
    const int maxConns = atoi(ArgGet(args, "--max", "4").c_str());
    const int secs = atoi(ArgGet(args, "--secs", "60").c_str());

    std::string allowJoined;
    for (size_t i = 0; i < allow.size(); ++i) {
        if (i) allowJoined += ",";
        allowJoined += allow[i];
    }

    SECURITY_ATTRIBUTES sa;
    ZeroMemory(&sa, sizeof(sa));
    sa.nLength = sizeof(sa);
    PSECURITY_DESCRIPTOR psd = NULL;
    if (!sddl.empty()) {
        if (!ConvertStringSecurityDescriptorToSecurityDescriptorA(
                sddl.c_str(), SDDL_REVISION_1, &psd, NULL)) {
            Result("SERVER-ERR", "ConvertStringSecurityDescriptor failed gle=%lu sddl=%s",
                   GetLastError(), sddl.c_str());
            return 2;
        }
        sa.lpSecurityDescriptor = psd;
    }

    const std::string full = "\\\\.\\pipe\\" + name;
    HANDLE h = CreateNamedPipeA(full.c_str(),
                                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                                1, 1024, 1024, 0, &sa);
    if (psd) LocalFree(psd);
    if (h == INVALID_HANDLE_VALUE) {
        Result("SERVER-ERR", "CreateNamedPipe failed gle=%lu pipe=%s", GetLastError(), full.c_str());
        return 2;
    }

    Result("SERVER-UP", "pipe=%s sddl=%s allow=%s max=%d lifetime=%ds pid=%lu serverSid=%s serverSession=%lu",
           name.c_str(),
           sddl.empty() ? "(default-DACL)" : sddl.c_str(),
           allowJoined.empty() ? "(none)" : allowJoined.c_str(),
           maxConns, secs,
           GetCurrentProcessId(),
           GetTokenSidString().c_str(),
           (unsigned long)CurrentSessionId());

    ULONGLONG deadline64 = GetTickCount64() + (ULONGLONG)secs * 1000;
    int conns = 0;
    while (conns < maxConns && GetTickCount64() < deadline64) {
        OVERLAPPED ov;
        ZeroMemory(&ov, sizeof(ov));
        ov.hEvent = CreateEventA(NULL, TRUE, FALSE, NULL);
        if (!ov.hEvent) break;
        BOOL connected = ConnectNamedPipe(h, &ov);
        if (!connected) {
            DWORD gle = GetLastError();
            if (gle == ERROR_IO_PENDING) {
                DWORD w = WaitForSingleObject(ov.hEvent, 2000);
                if (w == WAIT_TIMEOUT) {
                    CancelIo(h);
                    DisconnectNamedPipe(h);
                    CloseHandle(ov.hEvent);
                    continue;
                }
                connected = TRUE;
            } else if (gle == ERROR_PIPE_CONNECTED) {
                connected = TRUE;
            } else {
                Result("SERVER-ERR", "ConnectNamedPipe gle=%lu", gle);
                CloseHandle(ov.hEvent);
                break;
            }
        }
        CloseHandle(ov.hEvent);
        if (!connected) continue;
        conns++;

        DWORD clientPid = 0;
        std::string clientSid;
        const bool haveSid = GetClientSidString(h, clientSid, clientPid);

        char req[256] = {0};
        DWORD got = 0;
        (void)IoWithTimeout(h, req, sizeof(req) - 1, &got, false, 5000);

        const char* reply;
        if (!haveSid) {
            Result("SERVER", "conn#%d clientPid=%lu sid-lookup-FAILED gle=%lu -> REJECT",
                   conns, clientPid, GetLastError());
            reply = "DENIED sid-lookup-failed";
        } else {
            bool allowed = false;
            for (size_t i = 0; i < allow.size(); ++i) {
                if (_stricmp(allow[i].c_str(), clientSid.c_str()) == 0) { allowed = true; break; }
            }
            if (allowed) {
                Result("SERVER", "conn#%d clientPid=%lu clientSid=%s -> ACCEPT",
                       conns, clientPid, clientSid.c_str());
                reply = "OK identity-verified";
            } else {
                Result("SERVER", "conn#%d clientPid=%lu clientSid=%s -> TOKEN-REJECT (not in allow list)",
                       conns, clientPid, clientSid.c_str());
                reply = "DENIED token-not-authorized";
            }
        }
        DWORD wrote = 0;
        (void)IoWithTimeout(h, (void*)reply, (DWORD)strlen(reply), &wrote, true, 5000);
        FlushFileBuffers(h);
        DisconnectNamedPipe(h);

        DWORD hc = 0;
        (void)GetProcessHandleCount(GetCurrentProcess(), &hc);
        Result("SERVER", "post-conn#%d handleCount=%lu", conns, hc);
    }

    Result("SERVER-DONE", "conns=%d (max=%d, lifetime=%ds elapsed)", conns, maxConns, secs);
    CloseHandle(h);
    return 0;
}
