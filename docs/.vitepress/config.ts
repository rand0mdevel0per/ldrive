import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'LDrive',
  description: 'Linux.DO 社区分布式文件存储系统',
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: '首页', link: '/' },
      { text: '快速开始', link: '/guide/getting-started' },
      { text: 'API', link: '/api/' },
    ],
    sidebar: {
      '/guide/': [
        { text: '快速开始', link: '/guide/getting-started' },
        { text: '节点部署', link: '/guide/node-setup' },
        { text: '积分系统', link: '/guide/credits' },
      ],
      '/api/': [
        { text: 'REST API', link: '/api/rest' },
        { text: 'P2P 协议', link: '/api/p2p' },
      ],
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/rand0mdevel0per/ldrive' },
    ],
  },
});
