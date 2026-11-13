// common.cpp - token/SID/privilege utilities for the M0-T4 IPC identity PoC.
// All queries are read-only against the current process token.
#include "common.h"

#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

std::string GetTokenSidString() {
    HANDLE tok = NULL;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &tok)) return std::string();
    std::string out;
    DWORD len = 0;
    (void)GetTokenInformation(tok, TokenUser, NULL, 0, &len);
    if (GetLastError() == ERROR_INSUFFICIENT_BUFFER && len > 0) {
        PTOKEN_USER tu = (PTOKEN_USER)malloc(len);
        if (tu && GetTokenInformation(tok, TokenUser, tu, len, &len)) {
            char* s = NULL;
            if (ConvertSidToStringSidA(tu->User.Sid, &s)) {
                out = s;
                LocalFree(s);
            }
        }
        free(tu);
    }
    CloseHandle(tok);
    return out;
}

DWORD CurrentSessionId() {
    DWORD sid = 0;
    (void)ProcessIdToSessionId(GetCurrentProcessId(), &sid);
    return sid;
}

bool HasPrivilege(const char* privilegeName) {
    HANDLE tok = NULL;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &tok)) return false;
    bool found = false;
    DWORD len = 0;
    (void)GetTokenInformation(tok, TokenPrivileges, NULL, 0, &len);
    if (GetLastError() == ERROR_INSUFFICIENT_BUFFER && len > 0) {
        PTOKEN_PRIVILEGES tp = (PTOKEN_PRIVILEGES)malloc(len);
        if (tp && GetTokenInformation(tok, TokenPrivileges, tp, len, &len)) {
            for (DWORD i = 0; i < tp->PrivilegeCount; ++i) {
                char name[64] = {0};
                DWORD nameLen = sizeof(name);
                if (LookupPrivilegeNameA(NULL, &tp->Privileges[i].Luid, name, &nameLen) &&
                    _stricmp(name, privilegeName) == 0) {
                    found = true;
                    break;
                }
            }
        }
        free(tp);
    }
    CloseHandle(tok);
    return found;
}

std::string MakeRwSddl(const std::vector<std::string>& sids) {
    std::string s = "D:P";
    for (size_t i = 0; i < sids.size(); ++i) {
        s += "(A;;GRGW;;;" + sids[i] + ")";
    }
    return s;
}

void Result(const char* tag, const char* fmt, ...) {
    printf("[%s] ", tag);
    va_list ap;
    va_start(ap, fmt);
    vprintf(fmt, ap);
    va_end(ap);
    printf("\n");
    fflush(stdout);
}
