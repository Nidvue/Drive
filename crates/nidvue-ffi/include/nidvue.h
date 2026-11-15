/* SPDX-License-Identifier: AGPL-3.0-only */
/* Copyright (C) 2026 Nidvue */

#ifndef NIDVUE_H
#define NIDVUE_H

/*
 * Nidvue Drive C ABI.
 *
 * The Qt application links this library; no other consumer should need it.
 * Rules:
 *   - every function returns 0 on success, a negative code on failure;
 *   - strings returned through an out-pointer are freed with nidvue_string_free;
 *   - a panic inside the engine is contained and reported as NIDVUE_PANIC;
 *   - callbacks run on a Rust thread and must be marshalled to the UI thread.
 */

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define NIDVUE_OK 0
#define NIDVUE_ERR -1
#define NIDVUE_PANIC -2
#define NIDVUE_INVALID -3

typedef struct NidvueClient NidvueClient;

typedef void (*nidvue_event_cb)(const char *json, void *userdata);

/* Open a client over a state directory. Returns NULL on failure. */
NidvueClient *nidvue_client_new(const char *state_dir, char **err);

/* Free a client, unmounting first if needed. */
void nidvue_client_free(NidvueClient *client);

/* Exchange a Personal Login Token for a bearer token. */
int32_t nidvue_client_login(NidvueClient *client, const char *token, char **err);

/* Forget the stored token. */
int32_t nidvue_client_logout(NidvueClient *client, char **err);

/* Write the status JSON into *out (caller frees). */
int32_t nidvue_client_status_json(NidvueClient *client, char **out);

/* Pause or resume every sync pair. */
int32_t nidvue_client_set_paused(NidvueClient *client, int32_t paused, char **err);

/* Set the aggregate bandwidth cap in bytes/second (0 = unlimited). */
int32_t nidvue_client_set_bandwidth_limit(NidvueClient *client,
                                          uint64_t bytes_per_second, char **err);

/* Mount the account at mountpoint. */
int32_t nidvue_client_mount(NidvueClient *client, const char *mountpoint,
                            int32_t read_only, char **err);

/* Unmount. */
int32_t nidvue_client_unmount(NidvueClient *client);

/* Run one sync pass; writes a JSON array of reports into *out. */
int32_t nidvue_client_sync_now(NidvueClient *client, char **out, char **err);

/* Set the cache cap in bytes. */
int32_t nidvue_client_set_cache_limit(NidvueClient *client, uint64_t bytes, char **err);

/* Installed plugins as a JSON array into *out. */
int32_t nidvue_client_plugins_json(NidvueClient *client, char **out);

/* Run one sync pass for a single pair; its report JSON goes into *out. */
int32_t nidvue_client_sync_pair_now(NidvueClient *client, const char *name,
                                    char **out, char **err);

/* Sync pairs as a JSON array into *out. */
int32_t nidvue_client_sync_pairs_json(NidvueClient *client, char **out);

/* Add a sync pair and write the new list into *out. */
int32_t nidvue_client_sync_pair_add(NidvueClient *client, const char *name,
                                    const char *local, const char *remote,
                                    char **out, char **err);

/* Remove a sync pair by name and write the remaining list into *out. */
int32_t nidvue_client_sync_pair_remove(NidvueClient *client, const char *name,
                                       char **out, char **err);

/* Pause or resume one sync pair and write the new list into *out. */
int32_t nidvue_client_sync_pair_set_paused(NidvueClient *client, const char *name,
                                           int32_t paused, char **out, char **err);

/* Open conflicts as a JSON array into *out. */
int32_t nidvue_client_conflicts_json(NidvueClient *client, char **out);

/* Transfers seen so far as a JSON array into *out, oldest first. */
int32_t nidvue_client_transfers_json(NidvueClient *client, char **out);

/* Forget every recorded transfer row. */
int32_t nidvue_client_clear_transfers(NidvueClient *client);

/* Resolve a conflict: "keep_local", "keep_remote" or "keep_both". */
int32_t nidvue_client_resolve_conflict(NidvueClient *client, const char *id,
                                       const char *resolution, char **err);

/* Subscribe to typed events. One subscription per client. */
int32_t nidvue_client_subscribe(NidvueClient *client, nidvue_event_cb cb,
                                void *userdata);

/* The JSON schema for events and configuration into *out. */
int32_t nidvue_schema_json(char **out);

/* Free a string returned by this library. */
void nidvue_string_free(char *value);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* NIDVUE_H */
