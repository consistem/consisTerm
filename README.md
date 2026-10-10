<p align="center">
  <img src="assets/logo.png" alt="Logo do consisTerm: um anel de vidro com um prompt de terminal, do verde-água ao violeta" width="160">
</p>

<h1 align="center">consisTerm</h1>

<p align="center">
  <b>O terminal para InterSystems IRIS, feito para quem vive nele.</b><br>
  Leia globais, consulte dados, pergunte ao Claude sobre a saída, compartilhe comandos com a equipe — sem sair do prompt.<br>
  Windows · Linux · macOS
</p>

<p align="center">
  <img alt="Licença: MIT" src="https://img.shields.io/badge/licen%C3%A7a-MIT-blue">
  <img alt="Escrito em Rust" src="https://img.shields.io/badge/escrito%20em-Rust-orange">
  <img alt="Roda em Windows, Linux e macOS" src="https://img.shields.io/badge/roda%20em-Windows%20%7C%20Linux%20%7C%20macOS-555">
  <img alt="Sem conta, sem telemetria" src="https://img.shields.io/badge/sem%20conta-sem%20telemetria-2ea44f">
</p>

<p align="center">
  <b>Português</b> · <a href="README.en.md">English</a>
</p>

<p align="center">
  <a href="#tooltip-de-globais">Tooltip de globais</a> ·
  <a href="#autocompletar-e-consulta-de-dados">Autocompletar</a> ·
  <a href="#envie-para-o-claude">Claude</a> ·
  <a href="#macros-para-você-e-para-toda-a-equipe">Macros</a> ·
  <a href="#todos-os-shells-numa-janela-só">Shells</a> ·
  <a href="#e-tudo-mais">Tudo mais</a> ·
  <a href="#downloads">Downloads</a>
</p>

---

## Tooltip de globais

**Aponte para uma global e ela diz o que é.** Passe o mouse sobre qualquer
parte de uma linha de `zwrite`, de um `write` de um nó ou de uma listagem do
`^%G`, e o consisTerm mostra qual propriedade aquele subscrito ou aquela peça
`^` guarda, o tipo, o tamanho e a lista de valores permitidos — lidos das
classes que mapeiam a global, no namespace em que a sessão está.

A pergunta vai por uma sessão própria, nunca pela que você está usando, e pede
só a estrutura, nunca o valor de um registro. Só a primeira consulta de cada
global espera.

<p align="center">
  <img src="docs/images/global-tooltip.png" alt="O ponteiro sobre a terceira peça de uma linha de zwrite; um tooltip mostra a propriedade, o tipo e os valores permitidos" width="100%">
  <br><sub>Passe o mouse numa peça da global e leia o que ela significa.</sub>
</p>

## Autocompletar e consulta de dados

**Sugestões enquanto você digita, vindas do próprio namespace.** Comandos,
funções `$`, globais `^`, rotinas, nomes em `##class(` e, no modo SQL,
palavras-chave e tabelas. As globais vêm direto do namespace — incluindo as
mapeadas e as `^mtemp…` do IRISTEMP — e um prefixo com globais demais vira uma
linha por próxima letra (`^TG…`), que vai estreitando conforme você digita.

Dentro de `^GLOBAL(` vai além: nomeia o subscrito em que você está pela
documentação da global e oferece suas constantes, seus valores listados e os
subscritos que existem ali agora. Cima/Baixo escolhem, Tab aceita, Esc fecha.

<p align="center">
  <img src="docs/images/autocomplete.png" alt="Digitando dentro de ^GLOBAL( aparece a lista de subscritos existentes com o nome do subscrito acima" width="100%">
  <br><sub>Subscritos reais do banco, nomeados pela documentação da global.</sub>
</p>

## Envie para o Claude

**Pergunte sobre o que está na tela.** *Analisar com o Claude* abre uma sessão
do Claude Code numa aba própria, já com a saída do terminal no contexto, e
espera a sua pergunta — nada é perguntado por você. Escolha quanto enviar:
tudo, os últimos 10 comandos, os últimos 5 ou só a seleção.

<p align="center">
  <img src="docs/images/claude.png" alt="Uma aba do Claude Code ao lado de uma sessão IRIS, respondendo sobre um erro na saída" width="100%">
  <br><sub>Um erro na tela, e uma aba que já sabe dele.</sub>
</p>

Requer o [`claude`](https://claude.com/claude-code) no `PATH`.

## Macros para você e para toda a equipe

**Uma coleção de comandos a um clique ou um atalho.** As macros ficam em XML:
os `{{parâmetros}}` são pedidos antes de enviar, `confirm="true"` protege
qualquer coisa que grave, cada macro pode ter seu próprio atalho e
`hide_command="true"` mantém fora da interface um comando que leva senha. O
editor nas Configurações confere cada parâmetro com o comando.

**O arquivo da organização** é uma segunda coleção, somente leitura, que toda
cópia do consisTerm carrega ao lado da sua. Coloque `org-macros.xml` ao lado do
executável — ou aponte as Configurações para um compartilhamento — e a equipe
inteira tem os mesmos comandos, mantidos num lugar só. Há um modelo em
[`packaging/org-macros.xml`](packaging/org-macros.xml).

<p align="center">
  <img src="docs/images/macros.png" alt="O menu do botão direito do terminal com os grupos de macros da Organização e Pessoais" width="100%">
  <br><sub>As macros da equipe e as suas, lado a lado.</sub>
</p>

## Todos os shells numa janela só

**Não só IRIS.** Prompt de Comando, Windows PowerShell, PowerShell 7, Git Bash,
WSL ou o que o `/etc/shells` listar — cada um numa aba, ao lado das sessões
IRIS, dividido com elas, com o mesmo tema. Cada shell é um `.toml` em
`plugins/shells/`, escrito na primeira vez que o app o encontra instalado e seu
para editar depois. *Abrir consisTerm aqui*, no menu de pastas do Explorer,
abre um naquela pasta.

<p align="center">
  <img src="docs/images/shells.png" alt="Abas de uma sessão IRIS, do PowerShell e do Git Bash na mesma janela" width="100%">
  <br><sub>IRIS, PowerShell e Git Bash na mesma janela.</sub>
</p>

---

## E tudo mais

* **Abas e painéis divididos** — uma sessão por aba, cada uma com seu
  histórico de rolagem e log; divida à direita ou abaixo para uma segunda
  sessão na mesma aba. A aba leva o nome da instância e, se quiser, do
  namespace, acompanhando cada `ZN`.
* **Barra de título em qualquer lado** — topo, base, esquerda ou direita, como
  nos navegadores. Num lado, as abas viram uma coluna.
* **Terminal suspenso** *(Windows)* — um atalho global (F12 por padrão) traz a
  janela do topo ou da base da tela, como o Guake e o Yakuake, e a guarda de
  novo.
* **Linhas muito maiores que a janela** — o terminal informa uma margem de
  16384 colunas, então um `zwrite` de uma global larga chega inteiro.
* **Cores de ObjectScript** — globais, strings, macros, referências a classes
  e métodos, rotinas e comandos, incluindo as abreviações. Atento ao prompt,
  então texto comum nunca acende.
* **Edição no prompt** — Home/End, saltos por palavra, clique para posicionar o
  cursor, e digitar `"`, `(` ou `[` sobre uma seleção a envolve em vez de
  substituí-la.
* **Histórico de comandos que sobrevive à sessão**, e **Ctrl+F** em todo o
  histórico de rolagem.
* **Modo SQL** — `/sql` ou Ctrl+Shift+Q entra no shell SQL do IRIS, com cores
  de SQL enquanto estiver lá.
* **Utilitários do IRIS** — compilar um pacote, compilar um grupo de rotinas,
  gerar uma interface, com a linha exata mostrada antes do envio.
* **Zoom** — Ctrl + roda do mouse, Ctrl+Mais/Menos, pinça no trackpad; Ctrl+0
  volta ao normal.
* **Temas editáveis** — dezesseis embutidos, de Windows XP e Windows 98 (claro
  e escuro) a **Alto Contraste** escuro e claro e **Seguro para Daltônicos**
  escuro e claro, com a paleta Okabe–Ito. Duplique qualquer um e todas as cores
  são suas, inclusive o layout da barra de título.
* **Configurações com busca**, em português (o padrão) ou inglês, com escala da interface
  de 100% a 150% e uma escala separada para a barra de título.
* **Exportação e log** — tela ou histórico como texto ou HTML com cores; logs
  por sessão com senhas ocultadas e rotação.
* **Protetores de tela** — Matrix, um logo quicando, ou seu próprio texto ou
  imagem.
* **Login automático** — credenciais do cofre de credenciais do sistema, nunca
  de um arquivo.
* **Fechar para a bandeja**, **sempre visível** e **atualização automática**
  pelo proxy da própria máquina — nada é baixado sem você pedir.

---

## Downloads

➡️ **[Última versão](https://github.com/consistem/consisTerm/releases/latest)**

| Plataforma | Arquivo | Observações |
| --- | --- | --- |
| Windows x64 | `consisterm-<versão>-windows-x64.exe` | Portátil — sem instalador. Também em `.zip`. |
| Linux x86_64 | `consisterm-<versão>-linux-x86_64.AppImage` | `chmod +x` e execute. Requer glibc 2.35+ (Ubuntu 22.04, Debian 12, Fedora 36 ou mais novos). Também em `.tar.gz`. |
| macOS (Apple silicon e Intel) | `consisterm-<versão>-macos-universal.dmg` | Abra e arraste o consisTerm para Aplicativos. |

Cada versão traz seu `SHA256SUMS.txt`. Além disso, só é preciso uma instância
IRIS ou Caché para conectar — as instâncias locais são encontradas sozinhas.

---

## Onde estamos

Sendo honestos, na 0.1:

* **Sólido:** sessões IRIS e de shell no Windows, e tudo acima que não esteja
  marcado de outra forma.
* **Novo:** as versões para Linux e macOS. Compilam sem problemas, mas foram
  muito menos usadas que a do Windows — relatos são bem-vindos.
* **Só no Windows, por enquanto:** o terminal suspenso, a bandeja, o menu do
  Explorer e substituir o `Iristerm.exe` na bandeja do IRIS.
* **Sem assinatura digital:** o SmartScreen do Windows e o Gatekeeper do macOS
  vão perguntar antes da primeira execução. No macOS, clique com o botão
  direito no app e escolha *Abrir*.

---

## Configuração

Tudo fica na pasta de configuração da plataforma — `%APPDATA%\consisTerm`,
`~/.config/consisTerm` ou `~/Library/Application Support/consisTerm`. Defina
`CONSISTERM_CONFIG_DIR` para guardar em outro lugar, como ao lado de uma cópia
portátil.

| Arquivo | Para quê |
| --- | --- |
| `settings.toml` | Idioma, tema, fonte, janela, sessões, log, perfis |
| `macros.xml` | Suas macros pessoais |
| `history.txt` | Comandos digitados num prompt IRIS, para recuperar |
| `themes/*.toml` | Seus próprios temas |
| `plugins/shells/*.toml` | Um por shell, encontrado ou declarado |

Senhas nunca vão para um arquivo: ficam no Gerenciador de Credenciais do
Windows, no Keychain do macOS ou no Secret Service.

Vindo do **newIrisTerminal**? A primeira execução copia suas configurações,
macros, temas e histórico; a pasta antiga fica como estava.

## Compilar a partir do código

```sh
cargo build --release
cargo test
```

O host de plugins WASM fica atrás de uma feature:

```sh
cargo build --release --features plugins
```

No Windows sem o Visual Studio, o toolchain GNU precisa de um MinGW-w64
completo ao lado:

```powershell
winget install Rustlang.Rust.GNU
winget install BrechtSanders.WinLibs.POSIX.MSVCRT
```

O empacotamento das versões fica em [`packaging/`](packaging/) e está descrito
em [`docs/releasing.md`](docs/releasing.md).

## ⚠️ Atenção

Macros e utilitários do IRIS digitam numa sessão real, e os bancos `RDB*` são
compartilhados com toda a equipe. Dê `confirm="true"` a qualquer macro que
altere dados: o consisTerm então mostra o texto exato que vai enviar e espera um
sim explícito.

## Versões e licença

[ZeroVer](https://0ver.org) — a versão principal fica em zero. Licença MIT.

<p align="center"><sub>Feito na <a href="https://www.consistem.com.br">Consistem</a>.</sub></p>
