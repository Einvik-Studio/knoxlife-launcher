# knoxlife-launcher – regler for AI

**Offentlig repo.** Launcheren til Knox Life og spillerdelen av Java-modene. Koden er åpen å lese, men alle rettigheter
er forbeholdt (`LICENSE`). Planen, vedtakene og systemdokumentet (`docs/systemer/launcher.md`) ligger i det private
repoet `zomboid-rp`, og reglene der gjelder også her – med ett tillegg:

- **Ingenting internt her:** ingen adresser til servere eller tjeneren, ingen nøkler, passord eller tokens, ingen navn
  på personer, ingen lenker til private repoer, dokumenter eller hubben, og ingen felles-filer fra malen. Serverdelen av
  Java-modene ligger i `zomboid-rp`, aldri her.
- Aldri spillets egne filer (`projectzomboid.jar` eller utdrag av spillets kode) i repoet.
- Tekst spillerne ser, er på engelsk (også `README.md`). Kommentarer og navn i koden på norsk.
- Før hver commit: `cd launcher && cargo test && cargo clippy --all-targets`.
- `main` endres bare via PR, som i de andre repoene.
