/* SPDX-License-Identifier: AGPL-3.0-only */
/* Copyright (C) 2026 Nidvue */

/*
 * A C consumer of the public header, built and run by `cargo xtask ffi-smoke`.
 *
 * The header test checks that every symbol is declared; this checks that the
 * declarations are usable from C: right types, right constness, and a static
 * library that actually links.
 */

#include <stdio.h>
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

    char *conflicts = NULL;
    check("conflicts_json returns 0", nidvue_client_conflicts_json(client, &conflicts) == NIDVUE_OK);
    nidvue_string_free(conflicts);

    nidvue_client_free(client);

    if (failures == 0) {
        printf("+ ffi smoke: the C ABI compiles, links and answers\n");
    }
    return failures == 0 ? 0 : 1;
}
