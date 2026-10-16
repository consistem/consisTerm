<!--
  The body every release draft opens with. The notes are in Brazilian
  Portuguese, the language the people using consisTerm read. Novidades is the
  release being cut: the commit that bumps the version rewrites it, so the
  draft opens with it already written. {version} is replaced with the version
  number, and GitHub's generated list of changes follows this text under the
  headings in .github/release.yml.
-->
## Novidades

- **Novos temas, com a janela de cada época:** System e System Dark, no estilo
  do Mac clássico; SNES e SNES Dark, com a fonte pixelada do Super Nintendo; e
  Windows XP Dark. Tiger Aqua agora traz também a janela Aqua do Mac OS X, com
  os botões em gota.
- **Terminal drop-down sem piscar:** a janela não mostra mais um contorno branco
  antes de aparecer, e desce e sobe suavemente em qualquer velocidade de
  animação (Rápido, Normal ou Lento).
- **Redimensionar não duplica mais a tela:** arrastar a borda da janela para
  diminuir e aumentar não empilha mais cópias do conteúdo no histórico.
- **Ctrl+clique abre endereços web** escritos no terminal.

## Instalação

| arquivo | o que é |
| --- | --- |
| `consisterm-{version}-windows-x64.exe` | o executável para Windows; basta abrir. |
| `consisterm-{version}-windows-x64.zip` | o mesmo executável com o README. |
| `consisterm-{version}-linux-x86_64.AppImage` | Linux em um arquivo só: `chmod +x` e execute. |
| `consisterm-{version}-linux-x86_64.tar.gz` | o mesmo programa para Linux, como árvore de pastas. |
| `consisterm-{version}-macos-universal.dmg` | `consisTerm.app` para Apple silicon e Intel, macOS 11 ou mais novo. |
| `SHA256SUMS.txt` | os checksums de todos os arquivos acima. |

Quem já tem o consisTerm instalado recebe a atualização pelo próprio programa.
