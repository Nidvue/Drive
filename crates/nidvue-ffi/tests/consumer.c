/* SPDX-License-Identifier: AGPL-3.0-only */
/* Copyright (C) 2026 Nidvue */

/*
 * A C consumer of the public header, built and run by `cargo xtask ffi-smoke`.
 *
 * The header test checks that every symbol is declared; this checks that the
 * declarations are usable from C: right types, right constness, and a static
 * library that actually links.
 */

#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "nidvue.h"

static int failures;

static void check(const char *what, int ok)
{
    if (!ok) {
        printf("FAIL %s\n", what);
        ++failures;
    }
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        printf("usage: consumer <state-dir>\n");
        return 2;
    }

    char *schema = NULL;
    check("schema_json returns 0", nidvue_schema_json(&schema) == NIDVUE_OK);
    check("schema_json writes a document", schema != NULL && strlen(schema) > 100);
    check("schema mentions Status", schema != NULL && strstr(schema, "Status") != NULL);
    nidvue_string_free(schema);

    char *err = NULL;
    NidvueClient *client = nidvue_client_new(argv[1], &err);
    if (client == NULL) {
        printf("FAIL client_new: %s\n", err != NULL ? err : "(no message)");
        return 1;
    }

    char *status = NULL;
    check("status_json returns 0", nidvue_client_status_json(client, &status) == NIDVUE_OK);
    check("status carries the version", status != NULL && strstr(status, "\"version\"") != NULL);
    check("status carries the transfer list", status != NULL && strstr(status, "\"transfers\"") != NULL);
    check("status names the secret backend", status != NULL && strstr(status, "\"secret_backend\"") != NULL);
    nidvue_string_free(status);

    char *transfers = NULL;
    check("transfers_json returns 0", nidvue_client_transfers_json(client, &transfers) == NIDVUE_OK);
    check("transfers_json is an empty array", transfers != NULL && strcmp(transfers, "[]") == 0);
    nidvue_string_free(transfers);

    check("clear_transfers returns 0", nidvue_client_clear_transfers(client) == NIDVUE_OK);

    char *config = NULL;
    check("config_list_json returns 0",
          nidvue_client_config_list_json(client, &config, &err) == NIDVUE_OK);
    check("config_list_json carries the cache cap",
          config != NULL && strstr(config, "cache.max_size_bytes") != NULL);
    check("config_list_json carries the pair filters",
          config != NULL && strstr(config, "sync.pairs") != NULL);
    nidvue_string_free(config);

    char *cap = NULL;
    check("config_get returns 0",
          nidvue_client_config_get(client, "cache.max_size_bytes", &cap, &err) == NIDVUE_OK);
    check("config_get is a number", cap != NULL && cap[0] != '\0' && cap[0] != '"');
    nidvue_string_free(cap);

    check("config_set writes a value",
          nidvue_client_config_set(client, "cache.max_size_bytes", "1048576", &err) == NIDVUE_OK);
    char *after = NULL;
    check("config_set round-trips",
          nidvue_client_config_get(client, "cache.max_size_bytes", &after, &err) == NIDVUE_OK
              && after != NULL && strcmp(after, "1048576") == 0);
    nidvue_string_free(after);
    check("config_set refuses an unknown key",
          nidvue_client_config_set(client, "no.such.key", "1", &err) == NIDVUE_ERR);
    if (err != NULL) {
        nidvue_string_free(err);
        err = NULL;
    }

    char *peeked = NULL;
    check("config_peek returns 0",
          nidvue_config_peek(argv[1], "ui.start_minimised", &peeked, &err) == NIDVUE_OK);
    check("config_peek reads a bool", peeked != NULL && strcmp(peeked, "false") == 0);
    nidvue_string_free(peeked);

    check("config_set writes the startup preference",
          nidvue_client_config_set(client, "ui.start_minimised", "true", &err) == NIDVUE_OK);
    char *wanted = NULL;
    check("config_peek reads back what was written",
          nidvue_config_peek(argv[1], "ui.start_minimised", &wanted, &err) == NIDVUE_OK
              && wanted != NULL && strcmp(wanted, "true") == 0);
    nidvue_string_free(wanted);


    /* Autostart writes $XDG_CONFIG_HOME/autostart, so point it at the state
       directory this smoke owns instead of the runner's home. */
    setenv("XDG_CONFIG_HOME", argv[1], 1);
    check("autostart starts disabled", nidvue_autostart_enabled() == 0);
    check("autostart_set enables", nidvue_autostart_set("/usr/bin/nidvue-ui", 1, &err) == NIDVUE_OK);
    check("autostart_enabled reads it back", nidvue_autostart_enabled() == 1);
    check("autostart_set refuses an empty command",
          nidvue_autostart_set("", 1, &err) == NIDVUE_ERR);
    if (err != NULL) {
        nidvue_string_free(err);
        err = NULL;
    }
    check("autostart_set disables", nidvue_autostart_set(NULL, 0, &err) == NIDVUE_OK);
    check("autostart is disabled again", nidvue_autostart_enabled() == 0);

    char *conflicts = NULL;
    check("conflicts_json returns 0", nidvue_client_conflicts_json(client, &conflicts) == NIDVUE_OK);
    nidvue_string_free(conflicts);

    nidvue_client_free(client);

    if (failures == 0) {
        printf("+ ffi smoke: the C ABI compiles, links and answers\n");
    }
    return failures == 0 ? 0 : 1;
}
