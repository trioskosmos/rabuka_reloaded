# HF Space Deployment Helper
# This script automates the upload and push process.

$TOKEN = "$env:HF_TOKEN"
$REPO = "trioskosmos/rabukasim"
$HF_CLI = "C:\Users\trios\.local\bin\hf.exe"

Write-Host "Installing HF CLI..." -ForegroundColor Cyan
powershell -ExecutionPolicy ByPass -c "irm https://hf.co/cli/install.ps1 | iex"

Write-Host "Uploading core components..." -ForegroundColor Cyan
& $HF_CLI upload $REPO engine/ --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO cards/ --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO Dockerfile --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO README.md --repo-type=space --token $TOKEN
& $HF_CLI upload $REPO .gitattributes --repo-type=space --token $TOKEN

Write-Host "Uploading consolidated images (this may take a while)..." -ForegroundColor Cyan
& $HF_CLI upload $REPO web_ui/img/cards_webp/ --repo-type=space --token $TOKEN

Write-Host "Uploading remaining web_ui files..." -ForegroundColor Cyan
& $HF_CLI upload $REPO web_ui/ --repo-type=space --token $TOKEN

Write-Host "Pushing to Git main branch..." -ForegroundColor Cyan
git branch -m master main 2>$null
git push space main --force

Write-Host "Deployment Complete!" -ForegroundColor Green
