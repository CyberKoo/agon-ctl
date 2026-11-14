/* Hardware-free ABI smoke test. From the workspace root:
 *
 * cargo build --locked -p agon-osd-ffi
 * macOS:
 * cc -std=c11 -Wall -Wextra -Werror -Icrates/agon-osd-ffi/include \
 *    crates/agon-osd-ffi/examples/smoke.c target/debug/libagon_osd_ffi.a \
 *    -F/System/Library/PrivateFrameworks -framework CoreDisplay \
 *    -framework CoreFoundation -framework IOKit -framework CoreGraphics \
 *    -o target/agon-osd-ffi-smoke
 * Linux:
 * cc -std=c11 -Wall -Wextra -Werror -Icrates/agon-osd-ffi/include \
 *    crates/agon-osd-ffi/examples/smoke.c target/debug/libagon_osd_ffi.a \
 *    -lpthread -ldl -lm -o target/agon-osd-ffi-smoke
 *
 * target/agon-osd-ffi-smoke
 *
 * Creates an empty native session; does not enumerate displays or access hardware.
 */
#include "agon_osd.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Unlike assert(), checks remain active when built with NDEBUG. */
#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "check failed at line %d: %s\n", __LINE__, #condition); \
        return EXIT_FAILURE; \
    } \
} while (0)

int main(void)
{
    size_t i;
    size_t count = agon_feature_count();
    size_t hdr_index = count;
    size_t gamma_index = count;
    agon_feature_info_t feature = {0};
    agon_named_value_t named = {0};
    agon_named_value_list_t *named_values = NULL;
    agon_capability_list_t *capabilities = NULL;
    agon_display_info_t display = {0};
    agon_value_t value = {123, 456};
    agon_session_t *session = NULL;
    agon_display_list_t *list = NULL;
    agon_error_t *error = NULL;
    agon_error_t *first = NULL;
    const char *first_message;
    agon_support_path_t support_path = 99;
    uint8_t supported = 99;
    uint8_t capability = 99;

    CHECK(agon_abi_version() == AGON_ABI_VERSION);
    CHECK(count == 55);
    printf("ABI %u, %zu features\n", (unsigned)agon_abi_version(), count);
    for (i = 0; i < count; ++i) {
        CHECK(agon_feature_get(i, &feature) == AGON_OK);
        CHECK(feature.name != NULL && feature.name[0] != '\0');
        CHECK(feature.address != NULL && feature.address[0] != '\0');
        if (strcmp(feature.name, "hdr-mode") == 0) {
            hdr_index = i;
        }
        if (strcmp(feature.name, "gamma") == 0) {
            gamma_index = i;
        }
        printf("%s [%s]\n", feature.name, feature.address);
    }
    CHECK(hdr_index < count);
    CHECK(gamma_index < count);
    CHECK(agon_feature_support_path(gamma_index, &support_path) == AGON_OK);
    CHECK(support_path == AGON_SUPPORT_PATH_CAPABILITY_LIST);
    CHECK(agon_feature_support_path(hdr_index, &support_path) == AGON_OK);
    CHECK(support_path == AGON_SUPPORT_PATH_MAXIMUM);
    CHECK(agon_feature_support_path(0, &support_path) == AGON_OK);
    CHECK(support_path == AGON_SUPPORT_PATH_NOT_APPLICABLE);
    support_path = 99;
    CHECK(agon_feature_support_path(count, &support_path) == AGON_INVALID_ARGUMENT);
    CHECK(support_path == AGON_SUPPORT_PATH_NOT_APPLICABLE);
    CHECK(agon_feature_support_path(0, NULL) == AGON_INVALID_ARGUMENT);
    CHECK(agon_capability_list_count(NULL) == 0);
    CHECK(agon_capability_list_get(NULL, 0, &capability) == AGON_INVALID_ARGUMENT);
    CHECK(capability == 0);
    agon_capability_list_destroy(NULL);
    CHECK(agon_feature_named_values(hdr_index, 3, &named_values, &error) == AGON_OK);
    CHECK(error == NULL && agon_named_value_list_count(named_values) == 4);
    CHECK(agon_named_value_list_get(named_values, 1, &named) == AGON_OK);
    CHECK(strcmp(named.name, "hdr-picture") == 0 && named.value == 1);
    agon_named_value_list_destroy(named_values);
    named_values = NULL;
    CHECK(agon_feature_named_values(gamma_index, 0, &named_values, &error) == AGON_OK);
    CHECK(error == NULL && agon_named_value_list_count(named_values) == 6);
    CHECK(agon_named_value_list_get(named_values, 0, &named) == AGON_OK);
    CHECK(strcmp(named.name, "gamma1") == 0 && named.value == 0);
    agon_named_value_list_destroy(named_values);
    named_values = NULL;
    CHECK(agon_feature_named_values(count, 0, &named_values, &error) == AGON_INVALID_ARGUMENT);
    CHECK(named_values == NULL && error != NULL);
    agon_error_destroy(error);
    error = NULL;
    CHECK(agon_named_value_list_count(NULL) == 0);
    CHECK(agon_named_value_list_get(NULL, 0, &named) == AGON_INVALID_ARGUMENT);
    CHECK(named.name == NULL && named.value == 0);
    agon_named_value_list_destroy(NULL);

    /* A failed metadata call clears previously borrowed pointers. */
    CHECK(agon_feature_get(count, &feature) == AGON_INVALID_ARGUMENT);
    CHECK(feature.name == NULL && feature.address == NULL);
    CHECK(agon_feature_get(0, NULL) == AGON_INVALID_ARGUMENT);
    CHECK(agon_display_list_count(NULL) == 0);
    display.id = "old id";
    display.name = "old name";
    display.location = "old location";
    display.vendor_id = display.model_id = display.serial_number = 123;
    CHECK(agon_display_list_get(NULL, 0, &display) == AGON_INVALID_ARGUMENT);
    CHECK(display.id == NULL && display.name == NULL && display.location == NULL);
    CHECK(display.vendor_id == 0 && display.model_id == 0 && display.serial_number == 0);

    CHECK(agon_session_create(NULL, &first) == AGON_INVALID_ARGUMENT);
    CHECK(first != NULL);
    first_message = agon_error_message(first);
    CHECK(first_message != NULL && strstr(first_message, "session output") != NULL);

    CHECK(agon_session_refresh(NULL, &list, &error) == AGON_INVALID_ARGUMENT);
    CHECK(list == NULL && error != NULL);
    agon_error_destroy(error);
    error = NULL;

    CHECK(agon_read(NULL, "display", "brightness", &value, &error) == AGON_INVALID_ARGUMENT);
    CHECK(value.current == 0 && value.maximum == 0);
    CHECK(error != NULL && error != first);
    CHECK(strcmp(agon_error_message(error), "session is NULL") == 0);
    printf("Expected error: %s\n", agon_error_message(error));
    agon_error_destroy(error);
    error = NULL;
    /* Each error owns its message; freeing another error must not affect it. */
    CHECK(agon_error_message(first) == first_message);
    CHECK(strstr(first_message, "session output") != NULL);
    agon_error_destroy(first);
    first = NULL;

    value.current = 123;
    value.maximum = 456;
    CHECK(agon_set(NULL, "display", "brightness", 42, &value, NULL) == AGON_INVALID_ARGUMENT);
    CHECK(value.current == 0 && value.maximum == 0);
    value.current = 123;
    value.maximum = 456;
    CHECK(agon_probe(NULL, "display", "brightness", &supported, &value, &error) == AGON_INVALID_ARGUMENT);
    CHECK(supported == 0 && value.current == 0 && value.maximum == 0);
    CHECK(error != NULL);
    agon_error_destroy(error);
    error = NULL;

    CHECK(agon_feature_capabilities(NULL, "display", "color-gamut", &capabilities, &error) == AGON_INVALID_ARGUMENT);
    CHECK(capabilities == NULL && error != NULL);
    agon_error_destroy(error);
    error = NULL;

    CHECK(agon_read(NULL, NULL, NULL, NULL, NULL) == AGON_INVALID_ARGUMENT);
    CHECK(strcmp(agon_error_message(NULL), "") == 0);
    agon_session_destroy(NULL);
    agon_display_list_destroy(NULL);
    agon_error_destroy(NULL);
    CHECK(agon_session_create(&session, &error) == AGON_OK);
    CHECK(session != NULL && error == NULL);
    CHECK(agon_read(session, "missing", "brightness", &value, &error) == AGON_DISPLAY_NOT_FOUND);
    CHECK(error != NULL && value.current == 0 && value.maximum == 0);
    agon_error_destroy(error);
    error = NULL;
    CHECK(agon_feature_capabilities(session, "missing", "color-gamut", &capabilities, &error) == AGON_DISPLAY_NOT_FOUND);
    CHECK(capabilities == NULL && error != NULL);
    agon_error_destroy(error);
    agon_session_destroy(session);
    puts("Hardware-free C ABI smoke test passed.");
    return EXIT_SUCCESS;
}
