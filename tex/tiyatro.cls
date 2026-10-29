% tiyatro.cls — a XeLaTeX class for stage plays, part of Dactile.
% https://github.com/D4shwood/dactile
%
% Copyright (C) 2026 D4shwood
% This program is free software: you can redistribute it and/or modify it under
% the terms of the GNU General Public License as published by the Free Software
% Foundation, either version 3 of the License, or (at your option) any later
% version. See LICENSE.
%
%   \documentclass[draft]{tiyatro}   line numbers, 1.5 spacing, "TASLAK" on the cover
%   \documentclass[final]{tiyatro}   clean copy for print
%   nohash     # at a line's start is not a heading (#sahne, #perde, #önsöz, #notlar)
%   nonotes    leaves the director's notes (\notlar) out of this copy
%
% The class is split in two: tiyatro-core.sty (characters, lines, scenes, acts,
% the cast table) and tiyatro-theme.sty (page, fonts, words, cover). A different
% look only needs a different theme.

\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{tiyatro}[2026/10/29 v1.3.0 Dactile stage play class]

\newif\iftiyatrodraft
\tiyatrodrafttrue
\DeclareOption{draft}{\tiyatrodrafttrue}
\DeclareOption{final}{\tiyatrodraftfalse}
\newif\iftiyatrohash
\tiyatrohashtrue
\DeclareOption{nohash}{\tiyatrohashfalse}
\newif\iftiyatronotes
\tiyatronotestrue
\DeclareOption{nonotes}{\tiyatronotesfalse}
\DeclareOption*{\PassOptionsToClass{\CurrentOption}{book}}
\ProcessOptions\relax

% Latin Modern, LaTeX's default font, is missing from some TeX installs, and
% the base class and fontspec ask for it before the theme picks the real fonts.
% Where it is missing, those requests get Liberation Serif.
\IfFileExists{lmroman12-regular.otf}{}{%
  \DeclareFontFamily{TU}{lmr}{}%
  \DeclareFontShape{TU}{lmr}{m}{n}{<->"Liberation Serif"}{}%
  \DeclareFontShape{TU}{lmr}{m}{it}{<->"Liberation Serif/I"}{}%
  \DeclareFontShape{TU}{lmr}{bx}{n}{<->"Liberation Serif/B"}{}%
  \DeclareFontShape{TU}{lmr}{bx}{it}{<->"Liberation Serif/BI"}{}%
  \DeclareFontShape{TU}{lmr}{b}{n}{<->ssub * lmr/bx/n}{}%
  \DeclareFontShape{TU}{lmr}{m}{sl}{<->ssub * lmr/m/it}{}%
  \DeclareFontShape{TU}{lmr}{m}{sc}{<->ssub * lmr/m/n}{}%
}

\LoadClass[12pt,a4paper,oneside]{book}

% The theme first: names are measured in the font they are printed in.
\RequirePackage{tiyatro-theme}
\RequirePackage{tiyatro-core}
