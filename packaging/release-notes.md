<!--
  The body every release draft opens with. The notes are in Brazilian
  Portuguese, the language the people using consisTerm read. Novidades is the
  release being cut: the commit that bumps the version rewrites it, so the
  draft opens with it already written. {version} is replaced with the version
  number, and GitHub's generated list of changes follows this text under the
  headings in .github/release.yml.
-->
## Novidades

- **Diagnóstico:** uma nova página no topo das Configurações verifica o ambiente
  (namespaces do IRIS, contêineres e versões das imagens do Docker, e o que mais
  a sua empresa definir) e mostra cada item com uma luz de status. As
  verificações vêm de um arquivo XML da empresa, distribuído como as macros, com
  scripts em Python, PowerShell, shell ou ObjectScript. Quando uma verificação
  sabe corrigir o problema, o botão **Corrigir** roda a correção numa aba do
  terminal, à vista, e **Conferir** diz se ela funcionou.
- **Abas na barra de título da janela:** ao ligar a opção, uma animação mostra
  que a janela pode ser arrastada segurando os botões da barra (fechar,
  minimizar...).

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
