import pathlib
import sys
import unittest
from unittest import mock

SDK_ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SDK_ROOT))

import wyrmgrid_sdk


class WindowsTrustTests(unittest.TestCase):
    def test_only_server_authentication_roots_are_loaded(self):
        certificates = [
            (b"system", "x509_asn", True),
            (b"server", "x509_asn", {wyrmgrid_sdk.SERVER_AUTH_OID}),
            (b"signing", "x509_asn", {"1.3.6.1.5.5.7.3.3"}),
            (b"bundle", "pkcs_7_asn", True),
        ]
        with mock.patch.object(
            wyrmgrid_sdk.ssl,
            "DER_cert_to_PEM_cert",
            side_effect=lambda value: f"<{value.decode()}>",
        ):
            roots = wyrmgrid_sdk._windows_root_pem(certificates)

        self.assertEqual(roots, "<system><server>")

    def test_non_windows_uses_the_standard_verified_context(self):
        expected = object()
        with (
            mock.patch.object(wyrmgrid_sdk.sys, "platform", "linux"),
            mock.patch.object(
                wyrmgrid_sdk.ssl,
                "create_default_context",
                return_value=expected,
            ) as create_default_context,
        ):
            context = wyrmgrid_sdk._https_context()

        self.assertIs(context, expected)
        create_default_context.assert_called_once_with()

    def test_windows_context_uses_the_operating_system_roots(self):
        context = mock.Mock()
        with (
            mock.patch.object(wyrmgrid_sdk.sys, "platform", "win32"),
            mock.patch.object(
                wyrmgrid_sdk.ssl,
                "enum_certificates",
                return_value=[(b"root", "x509_asn", True)],
            ) as enum_certificates,
            mock.patch.object(
                wyrmgrid_sdk.ssl,
                "DER_cert_to_PEM_cert",
                return_value="<root>",
            ),
            mock.patch.object(
                wyrmgrid_sdk.ssl,
                "SSLContext",
                return_value=context,
            ) as ssl_context,
        ):
            result = wyrmgrid_sdk._https_context()

        self.assertIs(result, context)
        enum_certificates.assert_called_once_with("ROOT")
        ssl_context.assert_called_once_with(wyrmgrid_sdk.ssl.PROTOCOL_TLS_CLIENT)
        self.assertTrue(context.check_hostname)
        self.assertEqual(context.verify_mode, wyrmgrid_sdk.ssl.CERT_REQUIRED)
        context.load_verify_locations.assert_called_once_with(cadata="<root>")


if __name__ == "__main__":
    unittest.main()
