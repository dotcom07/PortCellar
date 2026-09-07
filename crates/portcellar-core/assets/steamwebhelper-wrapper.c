/*
 * Steam WebHelper wrapper for Wine/macOS.
 *
 * Derived from the MIT-licensed steamwebhelper-wrapper in
 * ramiabih/play-windows-steam-on-mac.
 */

#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif

#include <windows.h>
#include <stdlib.h>
#include <wchar.h>

#define REAL_BINARY L"steamwebhelper_real.exe"
#define WRAPPER_MARKER "portcellar-steamwebhelper-wrapper-v5"
#define EXTRA_FLAGS_SUFFIX \
    L"--disable-gpu " \
    L"--disable-features=IsolateOrigins,site-per-process,SpareRendererForSitePerProcess"
#define CROSSOVER_POLICY_ENV L"PORTCELLAR_STEAM_CEF_POLICY"
#define CROSSOVER_POLICY_VALUE L"crossover-compatible"
#define CROSSOVER_POLICY_FLAGS \
    L"--no-sandbox " \
    L"--in-process-gpu " \
    L"--use-angle=swiftshader-webgl " \
    L"--use-gl=angle " \
    L"--disable-component-update " \
    L"--gaia-url=http://disabled.invalid"
#define SINGLE_PROCESS_FLAG L"--single-process"

#if defined(__GNUC__)
__attribute__((used))
#endif
static const volatile char portcellar_wrapper_marker[] = WRAPPER_MARKER;

static wchar_t *resolve_real_binary(void)
{
    wchar_t self[MAX_PATH];
    DWORD len = GetModuleFileNameW(NULL, self, MAX_PATH);
    if (len == 0 || len >= MAX_PATH) return NULL;

    wchar_t *slash = wcsrchr(self, L'\\');
    if (!slash) return NULL;
    *(slash + 1) = L'\0';

    size_t cap = wcslen(self) + wcslen(REAL_BINARY) + 1;
    wchar_t *real = (wchar_t *)calloc(cap, sizeof(wchar_t));
    if (!real) return NULL;
    wcscpy(real, self);
    wcscat(real, REAL_BINARY);
    return real;
}

static const wchar_t *args_tail(void)
{
    const wchar_t *cmd = GetCommandLineW();
    if (!cmd) return L"";

    int in_quotes = 0;
    while (*cmd) {
        wchar_t c = *cmd;
        if (c == L'"') in_quotes = !in_quotes;
        else if (c == L' ' && !in_quotes) break;
        ++cmd;
    }
    while (*cmd == L' ') ++cmd;
    return cmd;
}

static int flag_enabled(const wchar_t *name)
{
    wchar_t value[16];
    DWORD len = GetEnvironmentVariableW(name, value, 16);
    if (len == 0 || len >= 16) return 0;

    return _wcsicmp(value, L"1") == 0 ||
           _wcsicmp(value, L"true") == 0 ||
           _wcsicmp(value, L"yes") == 0 ||
           _wcsicmp(value, L"on") == 0;
}

static int crossover_policy_enabled(void)
{
    wchar_t value[64];
    DWORD len = GetEnvironmentVariableW(CROSSOVER_POLICY_ENV, value, 64);
    if (len == 0 || len >= 64) return 0;

    return _wcsicmp(value, CROSSOVER_POLICY_VALUE) == 0;
}

int wmain(void)
{
    (void)portcellar_wrapper_marker;

    wchar_t *real = resolve_real_binary();
    if (!real) return 1;

    const wchar_t *tail = args_tail();
    int single_process = flag_enabled(L"PORTCELLAR_STEAM_CEF_SINGLE_PROCESS");
    int crossover_policy = crossover_policy_enabled();
    const wchar_t *policy_flags = crossover_policy ? CROSSOVER_POLICY_FLAGS : L"";
    size_t cap = wcslen(real) + wcslen(tail) + wcslen(EXTRA_FLAGS_SUFFIX) +
                 wcslen(policy_flags) +
                 (single_process ? wcslen(SINGLE_PROCESS_FLAG) + 1 : 0) + 8;
    wchar_t *cmdline = (wchar_t *)calloc(cap, sizeof(wchar_t));
    if (!cmdline) {
        free(real);
        return 1;
    }

    if (single_process && crossover_policy) {
        _snwprintf(
            cmdline,
            cap,
            L"\"%ls\" %ls %ls %ls %ls",
            real,
            tail,
            EXTRA_FLAGS_SUFFIX,
            policy_flags,
            SINGLE_PROCESS_FLAG
        );
    } else if (single_process) {
        _snwprintf(
            cmdline,
            cap,
            L"\"%ls\" %ls %ls %ls",
            real,
            tail,
            EXTRA_FLAGS_SUFFIX,
            SINGLE_PROCESS_FLAG
        );
    } else if (crossover_policy) {
        _snwprintf(
            cmdline,
            cap,
            L"\"%ls\" %ls %ls %ls",
            real,
            tail,
            EXTRA_FLAGS_SUFFIX,
            policy_flags
        );
    } else {
        _snwprintf(cmdline, cap, L"\"%ls\" %ls %ls", real, tail, EXTRA_FLAGS_SUFFIX);
    }

    STARTUPINFOW si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    if (!CreateProcessW(real, cmdline, NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi)) {
        free(cmdline);
        free(real);
        return 1;
    }

    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 0;
    GetExitCodeProcess(pi.hProcess, &code);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    free(cmdline);
    free(real);
    return (int)code;
}
