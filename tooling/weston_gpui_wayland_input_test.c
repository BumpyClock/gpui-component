/*
 * Conformance-only Weston client-test fixture for GPUI's Wayland input path.
 * Built inside the pinned Weston source tree; never installed.
 */

#include "config.h"

#include <errno.h>
#include <spawn.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>

#include "weston-test-fixture-compositor.h"
#include "weston-test-runner.h"

extern char **environ;

static enum test_result_code
fixture_setup(struct weston_test_harness *harness)
{
	struct compositor_setup setup;

	compositor_setup_defaults(&setup);
	setup.backend = WESTON_BACKEND_HEADLESS;
	setup.renderer = WESTON_RENDERER_PIXMAN;
	setup.shell = SHELL_TEST_DESKTOP;
	setup.width = 320;
	setup.height = 240;
	setup.refresh = 60000;
	setup.transform = WL_OUTPUT_TRANSFORM_NORMAL;

	return weston_test_harness_execute_as_client(harness, &setup);
}
DECLARE_FIXTURE_SETUP(fixture_setup);

static enum test_result_code
run_gpui_wayland_clipboard(struct wet_testsuite_data *suite_data)
{
	const char *session = getenv("GPUI_STAGE1_WAYLAND_CLIPBOARD_SESSION");
	char *const argv[] = { "bash", (char *) session, NULL };
	pid_t child;
	int status;
	int ret;

	(void) suite_data;

	if (!session || session[0] == '\0') {
		testlog("GPUI_STAGE1_WAYLAND_CLIPBOARD_SESSION is not set.\n");
		return RESULT_HARD_ERROR;
	}

	if (setenv("WAYLAND_DISPLAY", THIS_TEST_NAME, 1) < 0) {
		testlog("Failed to set WAYLAND_DISPLAY: %s.\n", strerror(errno));
		return RESULT_HARD_ERROR;
	}

	ret = posix_spawnp(&child, "bash", NULL, NULL, argv, environ);
	if (ret != 0) {
		testlog("Failed to spawn GPUI Wayland clipboard conformance: %s.\n", strerror(ret));
		return RESULT_HARD_ERROR;
	}

	do {
		ret = waitpid(child, &status, 0);
	} while (ret < 0 && errno == EINTR);
	if (ret < 0) {
		testlog("Failed to wait for GPUI Wayland clipboard conformance: %s.\n", strerror(errno));
		return RESULT_HARD_ERROR;
	}
	if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
		testlog("GPUI Wayland clipboard conformance failed with wait status %d.\n", status);
		return RESULT_FAIL;
	}

	return RESULT_OK;
}

DECLARE_TEST_LIST(TESTFN(run_gpui_wayland_clipboard));
