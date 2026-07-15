# Open-source licence notices

This source tree and the application **Open-source licences** view identify the
storage and cryptography components introduced by encrypted local data. Release
automation must distribute these notices with binaries; this page does not
replace the full dependency licence inventory produced for a release.

## SQLCipher Community Edition

Copyright (c) 2008-2020 Zetetic LLC

All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

- Redistributions of source code must retain the above copyright notice, this
  list of conditions and the following disclaimer.
- Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.
- Neither the name of ZETETIC LLC nor the names of its contributors may be used
  to endorse or promote products derived from this software without specific
  prior written permission.

THIS SOFTWARE IS PROVIDED BY ZETETIC LLC "AS IS" AND ANY EXPRESS OR IMPLIED
WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO
EVENT SHALL ZETETIC LLC BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT
OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING
IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
OF SUCH DAMAGE.

## OpenSSL

The bundled SQLCipher build uses OpenSSL 3 under the Apache License 2.0. The
authoritative licence text is available from the
[OpenSSL project](https://www.openssl.org/source/license.html) and is included
by the vendored `openssl-src` package. Release packaging must reproduce the
complete OpenSSL licence and applicable notices alongside the SQLCipher notice.

## Release gate

Before the first installable release, generate and review the complete Rust and
JavaScript dependency inventory, preserve required copyright and notice files,
make it reachable from the installed application and distribution archive, and
verify that the bundled source versions match the notices. CI licence policy is
a guardrail, not a substitute for this packaging check.
