param(
    [string]$BaseUrl = $env:ALMANAC_BASE_URL
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    throw "Set ALMANAC_BASE_URL or pass -BaseUrl. Example: .\scripts\smoke-production.ps1 -BaseUrl https://example.com"
}

$BaseUrl = $BaseUrl.TrimEnd("/")

function Assert-OkEnvelope {
    param(
        [string]$Name,
        [object]$Response
    )

    if ($Response.success -ne $true) {
        throw "$Name did not return a success envelope"
    }

    if ($Response.message -ne "OK") {
        throw "$Name returned unexpected message '$($Response.message)'"
    }
}

function Invoke-JsonGet {
    param(
        [string]$Name,
        [string]$Path
    )

    $url = "$BaseUrl$Path"
    Write-Host "GET $Path"
    $response = Invoke-RestMethod -Method Get -Uri $url -Headers @{ Accept = "application/json" }
    Assert-OkEnvelope -Name $Name -Response $response
    return $response
}

$root = Invoke-JsonGet -Name "root" -Path "/"
if ($root.data.base_path -ne "/api/v1") {
    throw "root returned unexpected base_path '$($root.data.base_path)'"
}

$health = Invoke-JsonGet -Name "health" -Path "/api/v1/health"
if ($health.data.status -ne "ok") {
    throw "health returned unexpected status '$($health.data.status)'"
}

$openapi = Invoke-RestMethod -Method Get -Uri "$BaseUrl/openapi.json" -Headers @{ Accept = "application/json" }
if (-not $openapi.openapi) {
    throw "openapi.json did not return an OpenAPI document"
}
Write-Host "GET /openapi.json"

$model = Invoke-JsonGet -Name "model detail" -Path "/api/v1/models/openai/gpt-4o"
if ($model.data.id -ne "gpt-4o") {
    throw "model detail returned unexpected model '$($model.data.id)'"
}

Write-Host "POST /api/v1/validate"
$validateBody = @{ model = "gpt-4o"; provider = "openai" } | ConvertTo-Json
$validate = Invoke-RestMethod `
    -Method Post `
    -Uri "$BaseUrl/api/v1/validate" `
    -ContentType "application/json" `
    -Body $validateBody `
    -Headers @{ Accept = "application/json" }
Assert-OkEnvelope -Name "validate" -Response $validate

Write-Host "Smoke checks passed for $BaseUrl"
