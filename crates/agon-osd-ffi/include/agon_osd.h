#ifndef AGON_OSD_H
#define AGON_OSD_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define AGON_ABI_VERSION 2u

typedef uint32_t agon_status_t;
#define AGON_OK 0u
#define AGON_INVALID_ARGUMENT 1u
#define AGON_UNSUPPORTED_PLATFORM 2u
#define AGON_DISPLAY_NOT_FOUND 3u
#define AGON_UNSUPPORTED_FEATURE 4u
#define AGON_VALUE_ABOVE_MAXIMUM 5u
#define AGON_TRANSPORT_ERROR 6u
#define AGON_INVALID_RESPONSE 7u
#define AGON_WRITE_UNCERTAIN 8u
#define AGON_READBACK_FAILED 9u
#define AGON_INTERNAL_ERROR 10u

typedef uint32_t agon_support_path_t;
#define AGON_SUPPORT_PATH_NOT_APPLICABLE 0u
#define AGON_SUPPORT_PATH_CAPABILITY_LIST 1u
#define AGON_SUPPORT_PATH_MAXIMUM 2u

typedef struct agon_session agon_session_t;
typedef struct agon_display_list agon_display_list_t;
typedef struct agon_named_value_list agon_named_value_list_t;
typedef struct agon_capability_list agon_capability_list_t;
typedef struct agon_error agon_error_t;

typedef struct {
    uint16_t current;
    uint16_t maximum;
} agon_value_t;

/* UTF-8 strings borrowed from the owning display list. */
typedef struct {
    const char *id;
    const char *name;
    const char *location;
    uint32_t vendor_id;
    uint32_t model_id;
    uint32_t serial_number;
} agon_display_info_t;

/* UTF-8 strings borrowed for the lifetime of the library. */
typedef struct {
    const char *name;
    const char *address;
} agon_feature_info_t;

typedef struct {
    const char *name;
    uint16_t value;
} agon_named_value_t;

/*
 * ABI v2. Native backends: macOS, Linux and Windows.
 * Functions are synchronous. Run device calls off the UI thread.
 * A session serializes hardware work on its own dedicated Rust thread.
 * Calls may share a session; destroy requires all callers to finish first.
 * Input strings must be valid NUL-terminated UTF-8 for the duration of the call.
 * Output pointers must be valid, writable and non-overlapping. Handles must be
 * live objects returned by this library, and freed exactly once using its API.
 * A non-NULL out_error receives NULL on success, or an owned error on failure.
 * Initialize/consume each output before reuse; the API does not free old outputs.
 * Destroy functions accept NULL. Never free Rust-owned objects with C free().
 */
uint32_t agon_abi_version(void);

/* Creates an empty session. Call refresh before device operations. */
agon_status_t agon_session_create(agon_session_t **out_session, agon_error_t **out_error);
void agon_session_destroy(agon_session_t *session);

/* Refresh replaces the session's device handles. Old lists remain readable snapshots. */
agon_status_t agon_session_refresh(agon_session_t *session,
                                   agon_display_list_t **out_list,
                                   agon_error_t **out_error);
size_t agon_display_list_count(const agon_display_list_t *list);
agon_status_t agon_display_list_get(const agon_display_list_t *list,
                                    size_t index,
                                    agon_display_info_t *out_info);
void agon_display_list_destroy(agon_display_list_t *list);

size_t agon_feature_count(void);
agon_status_t agon_feature_get(size_t index, agon_feature_info_t *out_info);
/* Returns AGON_SUPPORT_PATH_* without accessing a device. */
agon_status_t agon_feature_support_path(size_t feature_index,
                                        agon_support_path_t *out_path);

/*
 * Returns an owned table selected by maximum from agon_read/agon_probe.
 * Static features ignore maximum; numeric features return an empty list.
 * Names are borrowed from the list until agon_named_value_list_destroy.
 */
agon_status_t agon_feature_named_values(size_t feature_index,
                                        uint16_t maximum,
                                        agon_named_value_list_t **out_values,
                                        agon_error_t **out_error);
size_t agon_named_value_list_count(const agon_named_value_list_t *list);
agon_status_t agon_named_value_list_get(const agon_named_value_list_t *list,
                                        size_t index,
                                        agon_named_value_t *out_value);
void agon_named_value_list_destroy(agon_named_value_list_t *list);

/* display_id is an exact ID from refresh; feature_name is a name from feature_get. */
agon_status_t agon_read(agon_session_t *session,
                        const char *display_id,
                        const char *feature_name,
                        agon_value_t *out_value,
                        agon_error_t **out_error);
agon_status_t agon_set(agon_session_t *session,
                       const char *display_id,
                       const char *feature_name,
                       uint16_t value,
                       agon_value_t *out_value,
                       agon_error_t **out_error);
/* Probe returns AGON_OK with out_supported = 0 for an unsupported feature. */
agon_status_t agon_probe(agon_session_t *session,
                         const char *display_id,
                         const char *feature_name,
                         uint8_t *out_supported,
                         agon_value_t *out_value,
                         agon_error_t **out_error);

/*
 * Reads the device capability string and returns the feature's explicitly listed
 * values in device order, without local filtering or maximum clamping.
 * An empty list means the address has no explicit value list.
 * The returned list is independent of later calls and session destruction.
 */
agon_status_t agon_feature_capabilities(agon_session_t *session,
                                        const char *display_id,
                                        const char *feature_name,
                                        agon_capability_list_t **out_values,
                                        agon_error_t **out_error);
size_t agon_capability_list_count(const agon_capability_list_t *list);
agon_status_t agon_capability_list_get(const agon_capability_list_t *list,
                                       size_t index,
                                       uint8_t *out_value);
void agon_capability_list_destroy(agon_capability_list_t *list);

/* Error messages are UTF-8, borrowed until agon_error_destroy. NULL maps to "". */
const char *agon_error_message(const agon_error_t *error);
void agon_error_destroy(agon_error_t *error);

#ifdef __cplusplus
}
#endif
#endif
