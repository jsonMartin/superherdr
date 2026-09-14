# Distribution files

`install.sh` is the Superherdr shell installer published with each release. `agent-detection/` mirrors Herdr's agent-detection catalog, which Superherdr keeps aligned with its bundled manifests.

`install.ps1`, `install.cmd`, `latest.json`, and `preview.json` are inherited from Herdr. Superherdr does not publish or use them: its binary self-update is disabled, but the inherited update code still embeds and tests these files. They are kept so upstream merges stay simple. Do not point users at them.
