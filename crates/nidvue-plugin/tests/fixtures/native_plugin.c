/* SPDX-License-Identifier: AGPL-3.0-only */
/* Copyright (C) 2026 Nidvue */

/*
 * A minimal native plugin, compiled by the test that loads it. It speaks the
 * ABI in docs/plugin-guide.md and answers the `path_filter` hook.
 */

#include <stdlib.h>
#include <string.h>

static char *dup_text(const char *text)
{
    const size_t len = strlen(text);
    char *out = malloc(len + 1);
    if (out != NULL)
        memcpy(out, text, len + 1);
    return out;
}

unsigned nidvue_plugin_abi_version(void)
{
    return 1;
}

int nidvue_plugin_init(const char *config)
{
    /* The host hands over {"id":"..."}; a real plugin would read its options. */
    return config != NULL ? 0 : 1;
}

int nidvue_plugin_call(const char *hook, const char *payload, char **out)
{
    if (hook == NULL || out == NULL)
        return 1;
    if (strcmp(hook, "path_filter") == 0) {
        /* Skip anything under /Archive/cache, like the WASM example does. */
        if (payload != NULL && strstr(payload, "/Archive/cache") != NULL)
            *out = dup_text("{\"include\":false}");
        else
            *out = dup_text("{\"include\":true}");
    } else {
        *out = dup_text("{\"ok\":true}");
    }
    return *out != NULL ? 0 : 2;
}

void nidvue_plugin_free(char *out)
{
    free(out);
}
