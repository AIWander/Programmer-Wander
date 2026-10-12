# Signing the Programmer extension

Two independent signatures are possible. Neither is applied by default.

1. **Authenticode on `programmer.exe`** (what Windows SmartScreen and Defender look at). `pack.ps1
   -Sign` signs the exe inside each bundle before it is packed, by calling the signing script
   given in `-SignScript` (default `C:\CPC\scripts\sign-cpc.ps1`). That script uses Azure
   Trusted Signing: endpoint `https://eus.codesigning.azure.net`, account `AIWander`, profile
   `cpc-signing`, SHA-256 file digest, RFC 3161 timestamp `http://timestamp.acs.microsoft.com`.
   Prerequisites: Azure CLI on PATH with a current `az login` (or a service principal in
   `AZURE_CLIENT_ID` / `AZURE_TENANT_ID` / `AZURE_CLIENT_SECRET`), the TrustedSigning PowerShell
   module, and the signer role on the AIWander signing account.
2. **MCPB bundle signature** (what Claude Desktop shows as signed or unsigned). `mcpb sign`
   needs a PEM certificate and PEM private key. Azure Trusted Signing does not hand out private
   keys, so this needs a separate code-signing certificate exported as PEM. Pass it with
   `-McpbCert`, `-McpbKey` (and `-McpbIntermediate` if the chain needs it). Without them,
   `-Sign` signs the exe only and the bundle reports "Not signed".

## The two commands

Run from a PowerShell 7 prompt on the signing machine, in the repository root:

```powershell
az login
pwsh -NoProfile -File mcpb\pack.ps1 -Clean -X64 -Sign
```

With a PEM certificate for the bundle signature as well:

```powershell
pwsh -NoProfile -File mcpb\pack.ps1 -Clean -X64 -Sign -McpbCert C:\keys\mcpb-cert.pem -McpbKey C:\keys\mcpb-key.pem
```

The script stops if the exe signature is not `Valid` after signing. Afterwards check:

```powershell
mcpb info mcpb\dist\programmer-2.0.0-rc.1-win-arm64.mcpb
```

Signing changes the exe and the bundle, so `SHA256SUMS.txt` is rewritten by the same run; upload
the new files and the new `SHA256SUMS.txt` together.
