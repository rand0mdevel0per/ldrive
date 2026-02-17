<script>
  let hash = '';
  let downloading = false;

  async function download() {
    if (!hash) return;
    downloading = true;

    try {
      const res = await fetch(`http://localhost:3000/api/file/${hash}`);
      if (res.ok) {
        const blob = await res.blob();
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = hash;
        a.click();
      }
    } catch (e) {
      alert('下载失败');
    }
    downloading = false;
  }
</script>

<main>
  <h1>LDrive 文件下载</h1>
  <input type="text" bind:value={hash} placeholder="输入文件 Hash" />
  <button on:click={download} disabled={downloading}>
    {downloading ? '下载中...' : '下载'}
  </button>
</main>

<style>
  main { max-width: 600px; margin: 2rem auto; padding: 1rem; }
  input { width: 400px; }
  button { margin-left: 1rem; }
</style>
