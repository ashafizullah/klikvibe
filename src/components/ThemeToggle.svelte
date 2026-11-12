<script lang="ts">
  // Dark, light, or as the computer is set.
  import { settings, type ThemeChoice } from "../lib/settings.svelte";
  import { setTheme } from "../lib/theme.svelte";
  import { t } from "../lib/i18n/index.svelte";
  import Segmented from "./Segmented.svelte";

  const options = $derived<{ value: ThemeChoice; icon: string; title: string }[]>([
    { value: "dark", icon: "moon", title: t("theme.dark") },
    { value: "light", icon: "sun", title: t("theme.light") },
    { value: "system", icon: "monitor", title: t("theme.system") },
  ]);
</script>

<Segmented {options} value={settings.theme} onchange={setTheme} label={t("theme.label")} />
