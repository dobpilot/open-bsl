# Порядок `НайтиФайлы`

Замер выполнен 18.09.2026 на серверной 1С 8.3.27.2342,
`/S localhost/test`, Linux, прямым 1cv8 без Unica. Два независимых
сеанса `/tmp/open-bsl-server-measure-mf9q5oqz` и
`/tmp/open-bsl-server-measure-gq_xwdg0` завершились с кодом 0. После каждого
прогона CF/БД восстановлены, XML совпал побайтно. Проба создала
и удалила только свой уникальный временный каталог.

- [Сценарий](../../../tests/conformance/measure/filesystem/file-search-order.bsl),
  SHA-256 `dc73ff703084cf5b96971d30a5666980b635a14afa80d0ddffd6d96e72ef6bcd`.
- [Oracle](../../../tests/conformance/measure/filesystem/file-search-order.platform.txt),
  SHA-256 `30f48c9700ef789fe1a5af6d3d9a705ae97ea65db5e96e5db12fba07eb787b47`.

Оба прогона 1С и open-bsl выдали одинаковые восемь строк.
Корневой порядок `m.bin, a.txt, z.txt, m-dir, a-dir, z-dir` не
является лексикографическим: метод сохраняет порядок host и не сортирует
его. При рекурсии сначала выдаются все совпадения текущего уровня,
затем потомки каталогов в том же host-порядке. RU/EN-имена, `*` и
`*.txt` дали одинаковую семантику.

Изменение runtime не потребовалось: итеративный обход уже сохранял
порядок `read_dir` и укладывал дочерние каталоги в стек в обратном
виде. Новый conformance-тест сверяет весь oracle в
source/bytecode × plain/optimize. Прочие ошибки обхода, циклы ссылок и
`sfile` этим замером не закрываются.
