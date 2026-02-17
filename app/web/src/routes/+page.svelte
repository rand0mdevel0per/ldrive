<script>
  let file;
  let uploading = false;
  let result = '';

  async function upload() {
    if (!file) return;
    uploading = true;

    const formData = new FormData();
    formData.append('file', file);

    try {
      const res = await fetch('http://localhost:3000/api/upload', {
        method: 'POST',
        body: formData
      });
      const data = await res.json();
      result = `上传成功！Hash: ${data.hash}`;
    } catch (e) {
      result = '上传失败';
    }
    uploading = false;
  }
</script>

<main>
  <h1>LDrive 文件上传</h1>
  <input type="file" bind:files={file} />
  <button on:click={upload} disabled={uploading}>
    {uploading ? '上传中...' : '上传'}
  </button>
  {#if result}
    <p>{result}</p>
  {/if}
</main>

<style>
  main { max-width: 600px; margin: 2rem auto; padding: 1rem; }
  button { margin-left: 1rem; }
</style>
