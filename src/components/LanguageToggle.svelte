<script lang="ts">
  import { i18n, setLocale, type Locale } from "../lib/i18n/index.svelte";
  import Segmented from "./Segmented.svelte";

  const LOCALES: { value: Locale; label: string }[] = [
    { value: "en", label: "EN" },
    { value: "id", label: "ID" },
  ];
</script>

<Segmented options={LOCALES} value={i18n.locale} onchange={setLocale} label="Language" />
