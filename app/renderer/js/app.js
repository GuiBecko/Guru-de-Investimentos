window.guru.app.info().then((info) => {
  document.querySelector('[data-testid="core-version"]').textContent = info.coreVersion;
  document.querySelector('[data-testid="schema-version"]').textContent = String(info.schemaVersion);
});
