# Reference values for the unit tests of stamp-core/src/stats, from R.
# Run with: cd tests/reference && pixi run Rscript reference_values.R
# Each unit test names the R function and package version its values came from.

suppressMessages({library(DescTools); library(PMCMRplus); library(dunn.test); library(vegan); library(qvalue)})
options(digits=17)
cat("R", R.version.string, "DescTools", as.character(packageVersion("DescTools")), "PMCMRplus", as.character(packageVersion("PMCMRplus")), "dunn.test", as.character(packageVersion("dunn.test")), "vegan", as.character(packageVersion("vegan")), "qvalue", as.character(packageVersion("qvalue")), "\n")
tab <- function(s1, s2, t1, t2) matrix(c(s1, t1 - s1, s2, t2 - s2), 2)
P <- function(...) cat(..., "\n")
for (t in list(c(10,8,30,40), c(4000,5000,500000,1000000), c(3,1,4,4), c(10,13,30,40))) {
  m <- tab(t[1],t[2],t[3],t[4]); P("== table", t)
  P("chisq", chisq.test(m, correct=FALSE)$p.value, "chisq_yates", chisq.test(m, correct=TRUE)$p.value)
  P("fisher", fisher.test(m)$p.value, "less", fisher.test(m, alternative="less")$p.value, "greater", fisher.test(m, alternative="greater")$p.value)
  P("gtest", GTest(m, correct="none")$p.value, "gtest_yates", GTest(m, correct="yates")$p.value)
  P("prop_z two", prop.test(t[1:2], t[3:4], correct=FALSE)$p.value, "greater", prop.test(t[1:2], t[3:4], correct=FALSE, alternative="greater")$p.value, "less", prop.test(t[1:2], t[3:4], correct=FALSE, alternative="less")$p.value)
  lo <- phyper(t[1], t[3], t[4], t[1]+t[2]); hi <- phyper(t[1]-1, t[3], t[4], t[1]+t[2], lower.tail=FALSE)
  P("phyper_lower", lo, "phyper_upper", hi)
  for (meth in c("wald","waldcc","score")) { r <- BinomDiffCI(t[1],t[3],t[2],t[4], method=meth); P("diffci", meth, r[1], r[2], r[3]) }
  r <- OddsRatio(m, method="wald", conf.level=0.95); P("oddsratio_wald", r)
  r <- BinomRatioCI(t[1],t[3],t[2],t[4], method="katz.log"); P("ratio_katz", r)
}
# Exact permutation and parametric-bootstrap p-values for table 1.
s1<-10;s2<-8;n1<-30;n2<-40; obs <- s1/n1 - s2/n2; k <- s1+s2
x <- 0:k; d <- x/n1 - (k-x)/n2; pr <- dhyper(x, n1, n2, k)
P("perm_exact two", sum(pr[abs(d) >= abs(obs) - 1e-12]), "lower", sum(pr[d <= obs + 1e-12]), "upper", sum(pr[d >= obs - 1e-12]))
p0 <- k/(n1+n2); g <- outer(0:n1, 0:n2, function(a,b) a/n1 - b/n2); w <- outer(dbinom(0:n1,n1,p0), dbinom(0:n2,n2,p0))
P("boot_exact two", sum(w[abs(g) >= abs(obs) - 1e-12]), "lower", sum(w[g <= obs + 1e-12]), "upper", sum(w[g >= obs - 1e-12]))
# Group tests.
a <- c(5,4,6,4,3); b <- c(5,2,2,5,6,7)
for (ve in c(TRUE,FALSE)) { r <- t.test(a,b,var.equal=ve); P("ttest var.equal", ve, r$p.value, t.test(a,b,var.equal=ve,alternative="greater")$p.value, r$conf.int) }
a2 <- c(3.4,6.3,5.3,1.4,6.3,6.3); b2 <- c(3.5,6.4,5.2,1.3,6.4,6.2); r <- t.test(a2,b2); P("welch2", r$p.value, r$conf.int)
a3 <- 1:10; b3 <- (1:10)*10; r <- t.test(a3,b3); P("welch3", r$p.value, t.test(a3,b3,alternative="greater")$p.value, r$conf.int)
# ANOVA, Kruskal-Wallis, eta-squared.
anv <- function(gs) { y <- unlist(gs); g <- factor(rep(seq_along(gs), lengths(gs))); f <- aov(y ~ g); c(oneway.test(y ~ g, var.equal=TRUE)$p.value, EtaSq(f)[1]) }
P("anova1", anv(list(c(5,4,6,4,3), c(5,2,2,5,6,7), 1:7)))
P("anova2", anv(list(1:5, (1:5)*10, c(4,5,4), c(5,5,5))))
P("anova3", anv(list(c(5,4,5,4,5), c(6,5,6,5,6,5), c(700,800,700))))
P("eta1", anv(list(1:4, 2:4, 1:4))[2])
kw <- function(gs) { y <- unlist(gs); g <- factor(rep(seq_along(gs), lengths(gs))); kruskal.test(y, g)$p.value }
P("kw1", kw(list(c(5,4,6,4,3), c(5,2,2,5,6,7), 1:7)), "kw2", kw(list(1:7, 8:15, 16:22)))
# Post-hoc tests on A, B, C.
A <- 1:5; B <- (1:6)*10; C <- 1:7; y <- c(A,B,C); g <- factor(rep(1:3, c(5,6,7)))
f <- aov(y ~ g); tk <- TukeyHSD(f); P("tukey (j-i order: 2-1,3-1,3-2)"); print(tk$g, digits=17)
sc <- ScheffeTest(f); P("scheffe"); print(sc$g, digits=17)
gh <- gamesHowellTest(y, g); P("gameshowell p"); print(gh$p.value, digits=17)
ghci <- function(x1, x2) { n1<-length(x1);n2<-length(x2); v1<-var(x1)/n1; v2<-var(x2)/n2; df <- (v1+v2)^2/(v1^2/(n1-1)+v2^2/(n2-1)); se <- sqrt((v1+v2)/2); d <- mean(x1)-mean(x2); q <- qtukey(0.95, 3, df); c(d - q*se, d + q*se, ptukey(abs(d)/se, 3, df, lower.tail=FALSE)) }
P("gh_ci 1:2", ghci(A,B)); P("gh_ci 1:3", ghci(A,C)); P("gh_ci 2:3", ghci(B,C))
invisible(capture.output(dn <- dunn.test(y, g, method="bonferroni", altp=TRUE))); P("dunn comparisons", dn$comparisons); P("dunn P.adjusted", dn$altP.adjusted); P("dunn Z", dn$Z)
for (pr in list(c(1,2),c(1,3),c(2,3))) { x1 <- list(A,B,C)[[pr[1]]]; x2 <- list(A,B,C)[[pr[2]]]; r <- t.test(x1/100,x2/100); P("welch pair", pr, r$p.value, r$conf.int*100) }
# Corrections.
p <- c(0.01, 0.04, 0.03, 0.04, 0.9, 0.02)
for (m in c("bonferroni","holm","BH")) P("p.adjust", m, p.adjust(p, method=m))
up <- (1:200)/200
P("pi0est lambda=0.5", pi0est(up, lambda=0.5)$pi0, "pi0est lambda=0.8", pi0est(up, lambda=0.8)$pi0)
sp <- c(1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1, 0.3, 0.5, 0.7, 0.95)
P("qvalue pi0=0.6", qvalue(sp, pi0=0.6)$qvalues)
P("qvalue pi0=1 big", qvalue(c(0.2,0.9,0.99), pi0=1)$qvalues)
# Studentized range.
P("ptukey", ptukey(3.5, 3, 10), ptukey(2.0, 5, 20), ptukey(4.2, 4, 7), ptukey(5.0, 10, 60))
P("qtukey", qtukey(0.95, 3, 10), qtukey(0.99, 4, 7), qtukey(0.95, 5, 20), qtukey(0.90, 10, 60))
# Rarefaction and richness.
P("rarefy 3,2,1 at 2", rarefy(c(3,2,1), 2), "at 4", rarefy(c(3,2,1), 4))
P("rarefy big", rarefy(c(500, 300, 120, 50, 20, 5, 3, 1, 1), c(10, 100, 500)))
inc <- matrix(c(1,1,1, 1,1,0, 0,1,0, 1,0,0), nrow=3); P("specaccum exact", specaccum(inc, method="exact")$richness)
inc2 <- rbind(c(1,1,1,0,0), c(1,0,1,1,0), c(1,0,0,1,1), c(1,1,0,0,0), c(1,0,0,0,0))

# --- Studentized range: precise quantiles (qtukey is only accurate to ~4 decimals)
A <- 1:5; B <- (1:6)*10; C <- 1:7
ghci <- function(x1, x2) { n1<-length(x1);n2<-length(x2); v1<-var(x1)/n1; v2<-var(x2)/n2; df <- (v1+v2)^2/(v1^2/(n1-1)+v2^2/(n2-1)); se <- sqrt((v1+v2)/2); d <- mean(x1)-mean(x2)
  qroot <- uniroot(function(q) ptukey(q, 3, df) - 0.95, c(1, 20), tol = 1e-14)$root
  cat("df", df, "qtukey", qtukey(0.95,3,df), "root", qroot, "ptukey(qtukey)", ptukey(qtukey(0.95,3,df),3,df), "\n")
  cat("ci_root", d - qroot*se, d + qroot*se, "\n") }
ghci(A,B); ghci(A,C); ghci(B,C)
for (k in c(3,4,5,10)) for (df in c(5.3, 7, 10, 20, 60)) cat("ppf", k, df, uniroot(function(q) ptukey(q, k, df) - 0.95, c(1, 30), tol = 1e-14)$root, "\n")
cat("ptukey", ptukey(3.5, 3, 10), ptukey(2.0, 5, 20), ptukey(4.2, 4, 7), ptukey(5.0, 10, 60), ptukey(3.0, 3, 5.3), "\n")

# --- vegan richness estimates
P("chao1 counts 1,1,2,5,9", estimateR(c(1,1,2,5,9))["S.chao1"])
P("chao1 counts 1,1,1,3", estimateR(c(1,1,1,3))["S.chao1"])
inc <- matrix(c(1,1,1, 1,1,0, 0,1,0, 1,0,0), nrow=3); P("specaccum exact", specaccum(inc, method="exact")$richness)
inc2 <- rbind(c(1,1,1,0,0), c(1,0,1,1,0), c(1,0,0,1,1), c(1,1,0,0,0), c(1,0,0,0,0))
P("specpool", unlist(specpool(inc2)[c("Species","chao")]), "Q1", sum(colSums(inc2)==1), "Q2", sum(colSums(inc2)==2))
inc3 <- rbind(c(1,1,0,0), c(1,0,1,0), c(1,0,0,1))
P("specpool Q2=0", unlist(specpool(inc3)[c("Species","chao")]), "Q1", sum(colSums(inc3)==1), "Q2", sum(colSums(inc3)==2))

# --- Normal distribution

# --- Kruskal-Wallis with small groups (STAMP v2 refused groups below five)
kw2 <- function(gs) { y <- unlist(gs); g <- factor(rep(seq_along(gs), lengths(gs))); kruskal.test(y, g)$p.value }
cat("kw small", kw2(list(c(1,2,3,4), c(1,2,3,4,5))), kw2(list(c(1.5, 2.5), c(3, 4, 5), c(0.5))),
    kw2(list(c(1,1,2), c(2,3,3), c(3,4,4,4))), kw2(list(c(1), c(2))), "\n")

# --- White's non-parametric t-test (Metastats; White, Nagarajan and Pop 2009)
# The method is that of detect_DA_features.r (Metastats, last modified
# 4/14/2009; archived from metastats.cbcb.umd.edu): proportions of each
# sample's total, Welch's t statistic, B random relabellings of the samples,
# the null statistics of all frequently observed features pooled when either
# group has fewer than 8 samples, and fisher.test on the counts summed over
# each group for features seen fewer times than the group has samples.
# Instead of B random relabellings, every split of the samples into the two
# groups is enumerated (each is equally likely under a random permutation),
# giving the exact p-values the Monte Carlo ones estimate. "More extreme"
# means strictly greater; a split that reproduces the observed values gives
# the observed statistic up to rounding and is not counted (relative
# tolerance 1e-9, as in STAMP); no other split comes within 1e-6 of it.
white_t <- function(x, y) { d <- mean(x) - mean(y); s <- sqrt(var(x)/length(x) + var(y)/length(y)); if (s == 0) d/1e-6 else d/s }
white_case <- function(name, counts, n1) {
  n <- ncol(counts); n2 <- n - n1; totals <- colSums(counts); P <- sweep(counts, 2, totals, "/")
  s1 <- rowSums(counts[, 1:n1, drop=FALSE]); s2 <- rowSums(counts[, (n1+1):n, drop=FALSE])
  frequent <- which(s1 >= n1 | s2 >= n2); sparse <- which(s1 < n1 & s2 < n2)
  t_obs <- sapply(1:nrow(P), function(i) white_t(P[i, 1:n1], P[i, (n1+1):n]))
  splits <- combn(n, n1)
  null <- sapply(1:ncol(splits), function(k) { g <- splits[, k]; sapply(1:nrow(P), function(i) white_t(P[i, g], P[i, -g])) })
  if (!is.matrix(null)) null <- matrix(null, nrow=1)
  stopifnot(all(sapply(frequent, function(i) sapply(1:ncol(splits), function(k) { g <- splits[, k]; var(P[i, g]) + var(P[i, -g]) > 0 }))))
  P0 <- function(...) cat(name, ..., "\n")
  P0("samples", n1, n2, "splits", ncol(splits), "pooled", n1 < 8 || n2 < 8, "frequent", frequent, "sparse", sparse)
  for (i in 1:nrow(P)) {
    w <- t.test(P[i, 1:n1], P[i, (n1+1):n], var.equal=FALSE)
    P0("feature", i, "t", t_obs[i], "t.test", if (is.finite(w$statistic)) w$statistic else NA, "effect", 100*(mean(P[i, 1:n1]) - mean(P[i, (n1+1):n])))
  }
  near <- function(a, b) abs(a - b) > 1e-9 * abs(b) & abs(a - b) <= 1e-6 * abs(b)
  for (i in frequent) {
    t <- t_obs[i]
    pool <- if (n1 < 8 || n2 < 8) null[frequent, , drop=FALSE] else null[i, , drop=FALSE]
    stopifnot(!any(near(abs(pool), abs(t))), !any(near(pool, t)))
    two <- mean(abs(pool) > abs(t) * (1 + 1e-9)); one <- mean(pool > t + 1e-9 * abs(t))
    ties <- sum(abs(abs(pool) - abs(t)) <= 1e-9 * abs(t))
    if (n1 < 8 || n2 < 8) {
      H <- length(frequent)
      P0("exact", i, "one", one, "two", two, "ties", ties, "metastats_as_written", two * H / (H + 1))
    } else P0("exact", i, "one", one, "two", two, "ties", ties)
  }
  for (i in sparse) {
    N1 <- sum(totals[1:n1]); N2 <- sum(totals[(n1+1):n]); m <- matrix(c(s1[i], N1 - s1[i], s2[i], N2 - s2[i]), 2)
    ci <- BinomDiffCI(s1[i], N1, s2[i], N2, method="waldcc")
    P0("fisher", i, "two", fisher.test(m)$p.value, "one", min(fisher.test(m, alternative="less")$p.value, fisher.test(m, alternative="greater")$p.value),
       "waldcc", 100 * ci[1], 100 * ci[2], 100 * ci[3])
  }
}
# Four against five samples: the null statistics are pooled.
other <- function(m, totals) rbind(m, totals - colSums(m))
wa <- other(rbind(c(120, 135, 110, 128,  80, 95, 70, 88, 92),
                  c( 50,  62,  45,  58,  52, 60, 47, 49, 57),
                  c( 10,  14,   3,   9,  18, 25, 12, 20, 16),
                  c(  0,   1,   0,   0,   0,  0,  1,  0,  1),
                  c(  3,   0,   7,   1,   0,  2,  0,  0,  1)),
            c(1000, 1200, 900, 1100, 1000, 1300, 950, 1050, 1150))
white_case("white 4v5", wa, 4)
# Eight against nine samples: each feature has its own null distribution.
wb <- other(rbind(c(130, 142, 118, 125, 150, 137, 121, 133,  101,  95, 112,  88, 104,  99, 107,  92, 110),
                  c( 40,  52,  35,  47,  60,  38,  44,  51,   36,  42,  30,  39,  45,  33,  41,  37,  48),
                  c( 20,  25,  18,  30,  22,  27,  19,  24,   23,  21,  28,  19,  26,  24,  20,  29,  22),
                  c(  0,   1,   0,   0,   2,   0,   0,   1,    1,   0,   0,   0,   0,   1,   0,   0,   0),
                  c(  5,   0,   3,   9,   0,   2,   7,   1,    0,   4,   0,   0,   6,   1,   0,   0,   2)),
            c(1000, 1100, 950, 1050, 1200, 1020, 980, 1080,  1000, 990, 1010, 970, 1060, 1030, 1040, 960, 1100))
white_case("white 8v9", wb, 8)
