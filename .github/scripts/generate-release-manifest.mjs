import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';

const INCLUDED_EXTENSIONS = new Set(['.exe', '.msi', '.dmg', '.pkg', '.appimage', '.deb', '.rpm']);
const EXCLUDED_FILENAMES = new Set([
  'release-manifest.json',
  'checksums.txt',
  'nightly-release-manifest.json',
  'nightly-checksums.txt',
]);
const EXCLUDED_SUFFIXES = ['.sig', '.blockmap', '.spdx.json'];

function getOptionalEnv(name, fallback) {
  return process.env[name] || fallback;
}

function getExcludedFilenames() {
  const excluded = new Set(EXCLUDED_FILENAMES);
  const configuredManifest = process.env.RELEASE_MANIFEST_FILE;
  const configuredChecksums = process.env.RELEASE_CHECKSUMS_FILE;

  if (configuredManifest) {
    excluded.add(configuredManifest.toLowerCase());
  }

  if (configuredChecksums) {
    excluded.add(configuredChecksums.toLowerCase());
  }

  return excluded;
}

function getRequiredEnv(name) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }
  return value;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function getExtension(filename) {
  const lowerName = filename.toLowerCase();

  if (lowerName.endsWith('.appimage')) {
    return '.appimage';
  }

  const lastDotIndex = lowerName.lastIndexOf('.');
  if (lastDotIndex === -1) {
    return '';
  }

  return lowerName.slice(lastDotIndex);
}

function shouldIncludeAsset(asset) {
  const lowerName = asset.name.toLowerCase();
  const excludedFilenames = getExcludedFilenames();

  if (excludedFilenames.has(lowerName)) {
    return false;
  }

  if (EXCLUDED_SUFFIXES.some((suffix) => lowerName.endsWith(suffix))) {
    return false;
  }

  const extension = getExtension(asset.name);
  return INCLUDED_EXTENSIONS.has(extension);
}

function inferPlatform(filename) {
  const lowerName = filename.toLowerCase();
  const extension = getExtension(filename);

  if (extension === '.exe' || extension === '.msi') {
    return 'windows';
  }

  if (extension === '.dmg' || extension === '.pkg') {
    return 'macos';
  }

  if (extension === '.appimage' || extension === '.deb' || extension === '.rpm') {
    return 'linux';
  }

  if (lowerName.includes('windows') || lowerName.includes('win')) {
    return 'windows';
  }

  if (lowerName.includes('macos') || lowerName.includes('darwin') || lowerName.includes('osx')) {
    return 'macos';
  }

  if (lowerName.includes('linux')) {
    return 'linux';
  }

  return 'unknown';
}

function inferArch(filename) {
  const lowerName = filename.toLowerCase();

  if (/(aarch64|arm64)/.test(lowerName)) {
    return 'arm64';
  }

  if (/(x86_64|x64|amd64)/.test(lowerName)) {
    return 'x64';
  }

  if (/(i386|i686|x86)/.test(lowerName)) {
    return 'x86';
  }

  if (lowerName.includes('universal')) {
    return 'universal';
  }

  return 'unknown';
}

function inferKind(filename) {
  const extension = getExtension(filename);

  if (extension === '.exe' || extension === '.msi' || extension === '.dmg' || extension === '.pkg') {
    return 'installer';
  }

  if (extension === '.deb' || extension === '.rpm') {
    return 'package';
  }

  if (extension === '.appimage') {
    return 'portable';
  }

  return 'unknown';
}

function compareAssets(left, right) {
  return left.platform.localeCompare(right.platform)
    || left.arch.localeCompare(right.arch)
    || left.name.localeCompare(right.name);
}

async function githubJsonRequest(url, token) {
  const response = await fetch(url, {
    headers: {
      Accept: 'application/vnd.github+json',
      Authorization: `Bearer ${token}`,
      'User-Agent': 'openfootmanager-release-manifest',
      'X-GitHub-Api-Version': '2022-11-28',
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub API request failed (${response.status}): ${body}`);
  }

  return response.json();
}

function isFullSha(value) {
  return /^[0-9a-f]{40}$/i.test(value ?? '');
}

function parsePositiveInteger(value, fallback) {
  if (!value) {
    return fallback;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

async function listWorkflowRuns(repository, workflowFile, token, branch) {
  const searchParams = new URLSearchParams({ per_page: '100' });

  if (branch) {
    searchParams.set('branch', branch);
  }

  const url = `https://api.github.com/repos/${repository}/actions/workflows/${workflowFile}/runs?${searchParams.toString()}`;
  const payload = await githubJsonRequest(url, token);
  return Array.isArray(payload.workflow_runs) ? payload.workflow_runs : [];
}

async function waitForWorkflowRunsToComplete(repository, workflowFile, token, branch, headSha) {
  const pollIntervalMs = parsePositiveInteger(process.env.RELEASE_SOURCE_WORKFLOW_POLL_INTERVAL_MS, 10000);
  const timeoutMs = parsePositiveInteger(process.env.RELEASE_SOURCE_WORKFLOW_TIMEOUT_MS, 30 * 60 * 1000);
  const deadline = Date.now() + timeoutMs;
  const normalizedHeadSha = isFullSha(headSha) ? headSha.toLowerCase() : '';

  while (true) {
    const workflowRuns = await listWorkflowRuns(repository, workflowFile, token, branch);
    const activeRuns = workflowRuns.filter((run) => {
      if (!run.status || run.status === 'completed') {
        return false;
      }

      if (normalizedHeadSha) {
        return (run.head_sha ?? '').toLowerCase() === normalizedHeadSha;
      }

      return true;
    });

    if (activeRuns.length === 0) {
      return;
    }

    const runSummary = activeRuns
      .map((run) => `${run.id}:${run.status}:${run.head_branch ?? 'unknown'}`)
      .join(', ');

    console.log(`Waiting for ${workflowFile} to finish before generating the manifest: ${runSummary}`);

    if (Date.now() >= deadline) {
      throw new Error(`Timed out waiting for ${workflowFile} to complete.`);
    }

    await delay(pollIntervalMs);
  }
}

async function downloadAsset(url, token) {
  const response = await fetch(url, {
    headers: {
      Accept: 'application/octet-stream',
      Authorization: `Bearer ${token}`,
      'User-Agent': 'openfootmanager-release-manifest',
    },
    redirect: 'follow',
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`Asset download failed (${response.status}): ${body}`);
  }

  const arrayBuffer = await response.arrayBuffer();
  return Buffer.from(arrayBuffer);
}

async function resolveReleaseTag() {
  if (process.env.RELEASE_TAG) {
    return process.env.RELEASE_TAG;
  }

  const configContents = await readFile(new URL('../../src-tauri/tauri.conf.json', import.meta.url), 'utf8');
  const config = JSON.parse(configContents);
  return `v${config.version}`;
}

async function fetchReleaseByTag(repository, tag, token) {
  const url = `https://api.github.com/repos/${repository}/releases/tags/${tag}`;
  let lastError;

  for (let attempt = 1; attempt <= 6; attempt += 1) {
    try {
      const release = await githubJsonRequest(url, token);
      if (Array.isArray(release.assets) && release.assets.length > 0) {
        return release;
      }
      lastError = new Error(`Release ${tag} has no assets yet.`);
    } catch (error) {
      lastError = error;
    }

    await delay(5000);
  }

  throw lastError;
}

async function buildManifest() {
  const repository = getRequiredEnv('GITHUB_REPOSITORY');
  const token = getRequiredEnv('GITHUB_TOKEN');
  const releaseStream = getOptionalEnv('RELEASE_STREAM', 'stable');
  const manifestFilename = getOptionalEnv('RELEASE_MANIFEST_FILE', 'release-manifest.json');
  const checksumsFilename = getOptionalEnv('RELEASE_CHECKSUMS_FILE', 'checksums.txt');
  const sourceWorkflowFile = process.env.RELEASE_SOURCE_WORKFLOW_FILE;
  const sourceWorkflowBranch = process.env.RELEASE_SOURCE_WORKFLOW_BRANCH;
  const sourceWorkflowHeadSha = process.env.RELEASE_SOURCE_WORKFLOW_HEAD_SHA;

  if (sourceWorkflowFile) {
    await waitForWorkflowRunsToComplete(repository, sourceWorkflowFile, token, sourceWorkflowBranch, sourceWorkflowHeadSha);
  }

  const tag = await resolveReleaseTag();
  const release = await fetchReleaseByTag(repository, tag, token);
  const assets = [];

  for (const asset of release.assets.filter(shouldIncludeAsset)) {
    const fileBuffer = await downloadAsset(asset.browser_download_url, token);
    const sha256 = createHash('sha256').update(fileBuffer).digest('hex');

    assets.push({
      id: asset.id,
      name: asset.name,
      label: asset.label ?? '',
      size: asset.size,
      contentType: asset.content_type ?? 'application/octet-stream',
      downloadUrl: asset.browser_download_url,
      sha256,
      platform: inferPlatform(asset.name),
      arch: inferArch(asset.name),
      kind: inferKind(asset.name),
      extension: getExtension(asset.name),
    });
  }

  if (assets.length === 0) {
    throw new Error(`No downloadable release assets were found for ${tag}.`);
  }

  assets.sort(compareAssets);

  const manifest = {
    schemaVersion: 1,
    product: 'Openfoot Manager',
    repository,
    tag,
    version: tag.startsWith('v') ? tag.slice(1) : tag,
    prerelease: Boolean(release.prerelease),
    publishedAt: release.published_at ?? release.created_at ?? new Date().toISOString(),
    releaseName: release.name ?? tag,
    releaseUrl: release.html_url,
    notes: release.body ?? '',
    assets,
  };

  const checksumLines = assets.map((asset) => `${asset.sha256}  ${asset.name}`);

  await writeFile(manifestFilename, `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(checksumsFilename, `${checksumLines.join('\n')}\n`);
}

buildManifest().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
