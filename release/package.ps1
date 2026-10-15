# Builds the release's Windows zip (P49, ADR-0035), from the repository's
# root, once the web app and the release binaries are built:
#
#   npm --prefix web run build
#   cargo build --release -p microscope-server -p microscope-launcher --locked
#   ./release/package.ps1 -Version 0.1.0
#
# The zip holds the launcher and the server, which embeds the web app; the
# default configurations; the README, the licence and NOTICE; and the
# third-party licences of the Rust crates (cargo-about) and of the npm
# packages the web app bundles. It's written to release-out/.
param(
    [string]$Version = 'dev',
    # Where Cargo built the binaries.
    [string]$Target = 'target',
    [string]$Out = 'release-out'
)
$ErrorActionPreference = 'Stop'

$name = "microscope-$Version-windows-x86_64"
$stage = Join-Path $Out $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage 'configs') | Out-Null

Copy-Item (Join-Path $Target 'release/microscope-launcher.exe'), (Join-Path $Target 'release/microscope-server.exe') $stage
Copy-Item configs/*.json (Join-Path $stage 'configs')
Copy-Item README.md, LICENSE, NOTICE $stage

# The Rust crates' licences, from the sources Cargo has: cargo-about runs
# offline, and fails on a licence the configuration doesn't accept.
cargo about generate --offline --fail -c release/about.toml -o (Join-Path $stage 'THIRD-PARTY-RUST.txt') release/about.hbs
if ($LASTEXITCODE -ne 0) { throw 'cargo about failed' }
# The npm packages' licences, which the web app's build lists.
Copy-Item web/dist/third-party-licences.txt (Join-Path $stage 'THIRD-PARTY-NPM.txt')

$zip = Join-Path $Out "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Output "packaged $zip"
