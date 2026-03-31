import importlib.machinery
import importlib.util
import pathlib
import unittest
from unittest import mock


ROOT = pathlib.Path(__file__).resolve().parents[1]


def load_module():
    loader = importlib.machinery.SourceFileLoader("aproman_module", str(ROOT / "aproman"))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        raise RuntimeError("Failed to create import spec for aproman")
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


APROMAN = load_module()

PACTL_CARDS_OUTPUT = """Card #0
\tName: alsa_card.pci-0000_00_1f.3
\tActive Profile: output:analog-stereo
\tports:
\t\tanalog-output-lineout: Line Out
\t\t\tProperties:
\t\t\t\tport.type = "analog"
Card #1
\tName: alsa_card.pci-0000_01_00.1
\tActive Profile: output:hdmi-stereo
\tports:
\t\thdmi-output-0: HDMI / DisplayPort 1
\t\t\tProperties:
\t\t\t\tport.type = "hdmi"
"""


class FakeProcess:
    def __init__(self, lines):
        self.stdout = iter(lines)
        self._terminated = False

    def poll(self):
        return 0 if self._terminated else None

    def terminate(self):
        self._terminated = True

    def wait(self, timeout=None):
        return 0


class ApromanTests(unittest.TestCase):
    @mock.patch.object(APROMAN.subprocess, "check_output", return_value=PACTL_CARDS_OUTPUT)
    def test_detect_hdmi_card_uses_hdmi_port_type(self, _check_output):
        self.assertEqual("alsa_card.pci-0000_01_00.1", APROMAN.detect_hdmi_card())

    @mock.patch.object(APROMAN.subprocess, "check_output", return_value=PACTL_CARDS_OUTPUT)
    def test_get_active_profile_reads_card_block(self, _check_output):
        profile = APROMAN.get_active_profile("alsa_card.pci-0000_01_00.1")
        self.assertEqual("output:hdmi-stereo", profile)

    def test_positive_float_rejects_non_positive_values(self):
        self.assertEqual(3.5, APROMAN.positive_float("3.5"))
        with self.assertRaises(APROMAN.argparse.ArgumentTypeError):
            APROMAN.positive_float("0")
        with self.assertRaises(APROMAN.argparse.ArgumentTypeError):
            APROMAN.positive_float("-1")

    @mock.patch.object(APROMAN.time, "sleep")
    @mock.patch.object(APROMAN, "set_card_profile", side_effect=[True, True])
    @mock.patch.object(APROMAN.subprocess, "run")
    @mock.patch.object(APROMAN, "get_active_profile", return_value="pro-audio")
    @mock.patch("builtins.print")
    def test_cycle_profile_restarts_services_then_restores_profile(
        self,
        _print,
        _get_active_profile,
        run_mock,
        set_card_profile_mock,
        sleep_mock,
    ):
        APROMAN.cycle_profile("alsa_card.pci-0000_01_00.1", "output:hdmi-stereo")

        self.assertEqual(
            [
                mock.call(["systemctl", "--user", "restart", "pipewire.service"], check=True),
                mock.call(["systemctl", "--user", "restart", "pipewire-pulse.service"], check=True),
            ],
            run_mock.call_args_list,
        )
        self.assertEqual(
            [
                mock.call("alsa_card.pci-0000_01_00.1", "off"),
                mock.call("alsa_card.pci-0000_01_00.1", "output:hdmi-stereo"),
            ],
            set_card_profile_mock.call_args_list,
        )
        sleep_mock.assert_called_once_with(1.0)

    @mock.patch.object(
        APROMAN.subprocess,
        "Popen",
        return_value=FakeProcess(
            [
                "signal time=1 member=PrepareForSleep\n",
                "   boolean true\n",
                "signal time=2 member=PrepareForSleep\n",
                "   boolean false\n",
            ]
        ),
    )
    @mock.patch("builtins.print")
    def test_monitor_suspend_resume_only_invokes_callback_on_resume(self, _print, _popen):
        on_resume = mock.Mock()

        APROMAN.monitor_suspend_resume(3.0, on_resume)

        on_resume.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
