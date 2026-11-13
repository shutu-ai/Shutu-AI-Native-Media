// common.h - shared utilities for the M0-T4 IPC identity PoC.
// PoC evidence probe only (M0-T4 / R-02); NOT product code, NOT part of runtime/.
#pragma once

#include <windows.h>
#include <sddl.h>
#include <string>
#include <vector>

// Current process token user SID as string (e.g. "S-1-5-21-..."); empty string on failure.
std::string GetTokenSidString();

// Kernel SessionId of the calling process (0 = services / Session 0 isolation).
DWORD CurrentSessionId();

// True if the current process token holds the named privilege (present, regardless of enabled state).
bool HasPrivilege(const char* privilegeName);

// Builds a protect-only DACL SDDL string granting GENERIC_READ|GENERIC_WRITE to each SID:
//   D:P(A;;GRGW;;;sid1)(A;;GRGW;;;sid2)...
std::string MakeRwSddl(const std::vector<std::string>& sids);

// Machine-parseable tagged output: "[TAG] message".
void Result(const char* tag, const char* fmt, ...);

// server mode implementation (server.cpp).
int RunServer(const std::vector<std::string>& args);

// --- tiny argument helpers -------------------------------------------------
inline bool ArgHas(const std::vector<std::string>& a, const char* flag) {
    for (size_t i = 0; i < a.size(); ++i) { if (a[i] == flag) return true; }
    return false;
}

inline std::string ArgGet(const std::vector<std::string>& a, const char* key, const char* def = "") {
    for (size_t i = 0; i + 1 < a.size(); ++i) { if (a[i] == key) return a[i + 1]; }
    return std::string(def);
}

// Splits "a,b,c" on commas.
inline std::vector<std::string> ArgList(const std::string& csv) {
    std::vector<std::string> out;
    size_t start = 0;
    while (true) {
        size_t c = csv.find(',', start);
        if (c == std::string::npos) { out.push_back(csv.substr(start)); break; }
        out.push_back(csv.substr(start, c - start));
        start = c + 1;
    }
    return out;
}
