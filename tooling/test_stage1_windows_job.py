#!/usr/bin/env python3
"""Unit seams for the Windows atomic Job Object launcher."""

from __future__ import annotations

import ctypes
from ctypes import wintypes
import io
from pathlib import Path
import subprocess
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parent))
import stage1_windows_job as launcher  # noqa: E402


class FakeKernel32:
    def __init__(self, close_result: bool = True) -> None:
        self.close_result = close_result
        self.closed_handles: list[int] = []
        self.terminated_jobs: list[int] = []

    def CloseHandle(self, handle: int) -> bool:
        self.closed_handles.append(handle)
        return self.close_result

    def TerminateJobObject(self, handle: int, exit_code: int) -> bool:
        self.terminated_jobs.append(handle)
        return True


class FakeProcess:
    def __init__(self, kernel32: FakeKernel32, job_handle: int | None) -> None:
        self._kernel32 = kernel32
        self._stage1_job_handle = job_handle


class WindowsJobLauncherTests(unittest.TestCase):
    def test_job_list_attribute_value_is_prelaunch_input_attribute(self) -> None:
        self.assertEqual(launcher.PROC_THREAD_ATTRIBUTE_JOB_LIST, 0x0002000D)

    def test_process_membership_postcondition_accepts_contained_child(self) -> None:
        class MembershipKernel32:
            def IsProcessInJob(self, process: int, job: int, result: object) -> bool:
                ctypes.cast(result, ctypes.POINTER(wintypes.BOOL)).contents.value = True
                return True

        launcher._require_process_in_job(MembershipKernel32(), 1, 2)

    def test_process_membership_postcondition_rejects_uncontained_child(self) -> None:
        class MembershipKernel32:
            def IsProcessInJob(self, process: int, job: int, result: object) -> bool:
                return True

        with self.assertRaises(OSError):
            launcher._require_process_in_job(MembershipKernel32(), 1, 2)

    def test_close_job_releases_handle_and_clears_process_reference(self) -> None:
        kernel32 = FakeKernel32()
        process = FakeProcess(kernel32, 123)

        self.assertTrue(launcher.close_job(process))
        self.assertEqual(kernel32.closed_handles, [123])
        self.assertIsNone(process._stage1_job_handle)

    def test_close_job_keeps_handle_for_taskkill_fallback_when_close_fails(self) -> None:
        kernel32 = FakeKernel32(close_result=False)
        process = FakeProcess(kernel32, 123)

        self.assertFalse(launcher.close_job(process))
        self.assertEqual(kernel32.closed_handles, [123, 123])
        self.assertEqual(kernel32.terminated_jobs, [123])
        self.assertEqual(process._stage1_job_handle, 123)

    def test_wait_reports_timeout_without_windows_runtime(self) -> None:
        class TimeoutKernel32:
            def WaitForSingleObject(self, handle: int, milliseconds: int) -> int:
                self.handle = handle
                self.milliseconds = milliseconds
                return launcher.WAIT_TIMEOUT

        kernel32 = TimeoutKernel32()
        process = launcher.WindowsJobProcess(
            kernel32,
            1,
            7,
            io.BytesIO(),
            io.BytesIO(),
            2,
            ["scenario"],
        )
        with self.assertRaises(subprocess.TimeoutExpired):
            process.wait(0.1)
        process._handle = None
        process._stage1_job_handle = None


if __name__ == "__main__":
    unittest.main()
